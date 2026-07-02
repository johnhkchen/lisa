//! DAG computation module for Lisa Zellij plugin.
//!
//! This module provides on-demand DAG computation from ticket frontmatter.
//! The DAG is computed dynamically - there is no DAG file. This is a core
//! design principle: agents cannot corrupt what doesn't exist.

use std::collections::{HashMap, HashSet, VecDeque};

use crate::types::{Phase, Ticket, TicketId, TicketStatus};

/// Represents the dependency graph computed from ticket frontmatter.
///
/// The DAG is computed on-demand from ticket files. It holds:
/// - All tickets indexed by their ID
/// - Forward edges (depends_on): what a ticket depends on
/// - Reverse edges (blocks): what tickets are blocked by a given ticket
#[derive(Debug, Clone)]
pub struct Dag {
    /// All tickets in the DAG, indexed by ticket ID
    nodes: HashMap<TicketId, Ticket>,
    /// Forward edges: ticket_id -> set of ticket IDs it depends on
    depends_on: HashMap<TicketId, HashSet<TicketId>>,
    /// Reverse edges: ticket_id -> set of ticket IDs it blocks
    blocks: HashMap<TicketId, HashSet<TicketId>>,
}

/// Result of cycle detection
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CycleDetectionResult {
    /// No cycles detected in the graph
    NoCycle,
    /// A cycle was detected, contains the IDs of tickets in the cycle
    Cycle(Vec<TicketId>),
}

/// Errors that can occur during DAG operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DagError {
    /// A ticket references a dependency that doesn't exist
    MissingDependency {
        ticket_id: TicketId,
        missing_dep: TicketId,
    },
    /// The graph contains a cycle
    CycleDetected(Vec<TicketId>),
}

impl Dag {
    /// Creates a new empty DAG.
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            depends_on: HashMap::new(),
            blocks: HashMap::new(),
        }
    }

    /// Builds a DAG from a list of tickets.
    ///
    /// This parses the `depends_on` field (and optional `blocks` field) from
    /// each ticket's frontmatter and constructs the dependency graph. The DAG
    /// can be fully computed from `depends_on` alone -- reverse edges
    /// (blocked-by relationships) are derived automatically. If `blocks` is
    /// present it is also used, but it is not required. The graph is validated
    /// to ensure all referenced dependencies exist.
    ///
    /// # Arguments
    ///
    /// * `tickets` - An iterator of tickets to build the DAG from
    ///
    /// # Returns
    ///
    /// A `Result` containing the constructed DAG or an error if validation fails.
    pub fn from_tickets<I>(tickets: I) -> Result<Self, DagError>
    where
        I: IntoIterator<Item = Ticket>,
    {
        let mut dag = Self::new();

        // First pass: add all nodes
        for ticket in tickets {
            dag.nodes.insert(ticket.id.clone(), ticket);
        }

        // Second pass: build edges
        for ticket in dag.nodes.values() {
            let ticket_id = &ticket.id;

            // Process depends_on relationships
            for dep_id in &ticket.depends_on {
                // Validate that the dependency exists
                if !dag.nodes.contains_key(dep_id) {
                    return Err(DagError::MissingDependency {
                        ticket_id: ticket_id.clone(),
                        missing_dep: dep_id.clone(),
                    });
                }

                // Add forward edge: this ticket depends on dep_id
                dag.depends_on
                    .entry(ticket_id.clone())
                    .or_default()
                    .insert(dep_id.clone());

                // Add reverse edge: dep_id blocks this ticket
                dag.blocks
                    .entry(dep_id.clone())
                    .or_default()
                    .insert(ticket_id.clone());
            }

            // Process blocks relationships (the inverse of depends_on)
            for blocked_id in &ticket.blocks {
                // Validate that the blocked ticket exists
                if !dag.nodes.contains_key(blocked_id) {
                    return Err(DagError::MissingDependency {
                        ticket_id: ticket_id.clone(),
                        missing_dep: blocked_id.clone(),
                    });
                }

                // Add forward edge: blocked_id depends on this ticket
                dag.depends_on
                    .entry(blocked_id.clone())
                    .or_default()
                    .insert(ticket_id.clone());

                // Add reverse edge: this ticket blocks blocked_id
                dag.blocks
                    .entry(ticket_id.clone())
                    .or_default()
                    .insert(blocked_id.clone());
            }
        }

        Ok(dag)
    }

    /// Returns a reference to the ticket with the given ID, if it exists.
    pub fn get_ticket(&self, id: &TicketId) -> Option<&Ticket> {
        self.nodes.get(id)
    }

    /// Returns an iterator over all tickets in the DAG.
    pub fn tickets(&self) -> impl Iterator<Item = &Ticket> {
        self.nodes.values()
    }

    /// Returns the number of tickets in the DAG.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Returns true if the DAG contains no tickets.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Gets all tickets that the given ticket depends on.
    pub fn get_dependencies(&self, ticket_id: &TicketId) -> HashSet<TicketId> {
        self.depends_on.get(ticket_id).cloned().unwrap_or_default()
    }

    /// Gets all tickets that are blocked by the given ticket.
    ///
    /// These are tickets that cannot start until the given ticket is done.
    pub fn get_blocked_by(&self, ticket_id: &TicketId) -> HashSet<TicketId> {
        self.blocks.get(ticket_id).cloned().unwrap_or_default()
    }

    /// Determines if a ticket can start work.
    ///
    /// A ticket can start if:
    /// 1. All tickets in its `depends_on` list are in the "done" phase
    /// 2. The ticket itself is in the "ready" phase (or appropriate starting phase)
    ///
    /// # Arguments
    ///
    /// * `ticket_id` - The ID of the ticket to check
    ///
    /// # Returns
    ///
    /// `true` if the ticket can start, `false` otherwise.
    /// Returns `false` if the ticket doesn't exist.
    pub fn can_start(&self, ticket_id: &TicketId) -> bool {
        let ticket = match self.nodes.get(ticket_id) {
            Some(t) => t,
            None => return false,
        };

        // Check if the ticket is in a startable phase
        if !ticket.phase.is_startable() {
            return false;
        }

        // Respect explicit blocked status
        if ticket.status == TicketStatus::Blocked {
            return false;
        }

        // Check if all transitive ancestors are done
        self.all_ancestors_done(ticket_id)
    }

    /// Checks if all dependencies of a ticket are in the "done" phase.
    pub fn all_dependencies_done(&self, ticket_id: &TicketId) -> bool {
        let deps = match self.depends_on.get(ticket_id) {
            Some(deps) => deps,
            None => return true, // No dependencies means all are satisfied
        };

        for dep_id in deps {
            if let Some(dep_ticket) = self.nodes.get(dep_id) {
                if dep_ticket.phase != Phase::Done {
                    return false;
                }
            } else {
                // Dependency doesn't exist in the DAG - shouldn't happen if
                // DAG was built correctly, but be defensive
                return false;
            }
        }

        true
    }

    /// Checks if all transitive ancestors of a ticket are in the "done" phase.
    ///
    /// Unlike `all_dependencies_done` which only checks direct deps, this walks
    /// the full transitive closure via BFS. This prevents scheduling a ticket
    /// when an intermediate dependency was falsely marked done while its own
    /// dependencies are still open.
    pub fn all_ancestors_done(&self, ticket_id: &TicketId) -> bool {
        let mut queue = VecDeque::new();
        let mut visited = HashSet::new();

        // Seed with direct dependencies
        if let Some(deps) = self.depends_on.get(ticket_id) {
            for dep in deps {
                if visited.insert(dep) {
                    queue.push_back(dep);
                }
            }
        }

        while let Some(ancestor_id) = queue.pop_front() {
            match self.nodes.get(ancestor_id) {
                Some(ticket) if ticket.phase == Phase::Done => {
                    // This ancestor is done — continue to check *its* ancestors
                    if let Some(deps) = self.depends_on.get(ancestor_id) {
                        for dep in deps {
                            if visited.insert(dep) {
                                queue.push_back(dep);
                            }
                        }
                    }
                }
                Some(_) => return false, // Ancestor not done
                None => return false,    // Missing ancestor — be defensive
            }
        }

        true
    }

    /// Gets all tickets that are ready to run.
    ///
    /// A ticket is ready to run if:
    /// - It has no unmet dependencies (all depends_on tickets are "done")
    /// - Its phase allows it to start (is in "ready" phase)
    /// - Its status allows work (is "open")
    ///
    /// # Returns
    ///
    /// A vector of ticket IDs that are ready to run.
    pub fn get_ready_tickets(&self) -> Vec<TicketId> {
        let mut ready: Vec<TicketId> = self
            .nodes
            .keys()
            .filter(|id| self.can_start(id))
            .cloned()
            .collect();
        ready.sort();
        ready
    }

    /// Gets all tickets currently in progress (active phases).
    ///
    /// This includes tickets in Research, Design, Structure, Plan, Implement,
    /// or Review phases.
    pub fn get_in_progress_tickets(&self) -> Vec<TicketId> {
        self.nodes
            .values()
            .filter(|t| t.phase.is_active())
            .map(|t| t.id.clone())
            .collect()
    }

    /// Detects cycles in the dependency graph.
    ///
    /// Uses Kahn's algorithm (topological sort) to detect cycles.
    /// If the algorithm cannot process all nodes, a cycle exists.
    ///
    /// # Returns
    ///
    /// `CycleDetectionResult::NoCycle` if no cycles exist, or
    /// `CycleDetectionResult::Cycle` with the IDs of tickets involved.
    pub fn detect_cycles(&self) -> CycleDetectionResult {
        if self.nodes.is_empty() {
            return CycleDetectionResult::NoCycle;
        }

        // Calculate in-degree for each node
        let mut in_degree: HashMap<&TicketId, usize> = HashMap::new();
        for id in self.nodes.keys() {
            in_degree.insert(id, 0);
        }

        // For each ticket and its dependencies, the ticket has incoming edges
        // from its dependencies
        for (ticket_id, deps) in &self.depends_on {
            // ticket_id depends on deps, so there are edges from each dep to ticket_id
            // This means ticket_id's in-degree increases by the number of deps
            if let Some(count) = in_degree.get_mut(ticket_id) {
                *count = deps.len();
            }
        }

        // Queue of nodes with no incoming edges
        let mut queue: VecDeque<&TicketId> = VecDeque::new();
        for (id, &degree) in &in_degree {
            if degree == 0 {
                queue.push_back(id);
            }
        }

        let mut processed = 0;
        let mut sorted: Vec<&TicketId> = Vec::new();

        while let Some(id) = queue.pop_front() {
            processed += 1;
            sorted.push(id);

            // For each ticket that this ticket blocks (reverse edges)
            if let Some(blocked) = self.blocks.get(id) {
                for blocked_id in blocked {
                    if let Some(count) = in_degree.get_mut(blocked_id) {
                        *count -= 1;
                        if *count == 0 {
                            queue.push_back(blocked_id);
                        }
                    }
                }
            }
        }

        if processed == self.nodes.len() {
            CycleDetectionResult::NoCycle
        } else {
            // Find nodes that weren't processed - they're in cycles
            let cycle_nodes: Vec<TicketId> = self
                .nodes
                .keys()
                .filter(|id| !sorted.contains(id))
                .cloned()
                .collect();
            CycleDetectionResult::Cycle(cycle_nodes)
        }
    }

    /// Performs a topological sort of all tickets.
    ///
    /// Returns tickets in an order where all dependencies come before
    /// the tickets that depend on them.
    ///
    /// # Returns
    ///
    /// `Ok` with a vector of ticket IDs in topological order, or
    /// `Err` with a `DagError::CycleDetected` if the graph has cycles.
    pub fn topological_sort(&self) -> Result<Vec<TicketId>, DagError> {
        if self.nodes.is_empty() {
            return Ok(Vec::new());
        }

        // Calculate in-degree for each node
        let mut in_degree: HashMap<&TicketId, usize> = HashMap::new();
        for id in self.nodes.keys() {
            in_degree.insert(id, 0);
        }

        for (ticket_id, deps) in &self.depends_on {
            if let Some(count) = in_degree.get_mut(ticket_id) {
                *count = deps.len();
            }
        }

        // Queue of nodes with no incoming edges
        let mut queue: VecDeque<&TicketId> = VecDeque::new();
        for (id, &degree) in &in_degree {
            if degree == 0 {
                queue.push_back(id);
            }
        }

        let mut result: Vec<TicketId> = Vec::new();

        while let Some(id) = queue.pop_front() {
            result.push(id.clone());

            // For each ticket that this ticket blocks
            if let Some(blocked) = self.blocks.get(id) {
                for blocked_id in blocked {
                    if let Some(count) = in_degree.get_mut(blocked_id) {
                        *count -= 1;
                        if *count == 0 {
                            queue.push_back(blocked_id);
                        }
                    }
                }
            }
        }

        if result.len() == self.nodes.len() {
            Ok(result)
        } else {
            // Find nodes that weren't processed - they're in cycles
            let cycle_nodes: Vec<TicketId> = self
                .nodes
                .keys()
                .filter(|id| !result.contains(id))
                .cloned()
                .collect();
            Err(DagError::CycleDetected(cycle_nodes))
        }
    }

    /// Gets the tickets that can be run concurrently at the current state.
    ///
    /// This returns all tickets that:
    /// 1. Have all dependencies satisfied (in "done" phase)
    /// 2. Are in a startable phase themselves
    ///
    /// These are the tickets that Lisa can spawn threads for simultaneously.
    pub fn get_runnable_tickets(&self) -> Vec<&Ticket> {
        self.nodes
            .values()
            .filter(|t| self.can_start(&t.id))
            .collect()
    }

    /// Computes the critical path through the DAG.
    ///
    /// The critical path is the longest chain of dependencies.
    /// This is useful for estimating minimum completion time.
    ///
    /// # Returns
    ///
    /// A vector of ticket IDs representing the longest dependency chain.
    pub fn critical_path(&self) -> Vec<TicketId> {
        if self.nodes.is_empty() {
            return Vec::new();
        }

        // Get topological order (or return empty if there's a cycle)
        let topo_order = match self.topological_sort() {
            Ok(order) => order,
            Err(_) => return Vec::new(),
        };

        // For each node, compute the longest path ending at that node
        let mut longest_path: HashMap<&TicketId, Vec<TicketId>> = HashMap::new();

        for id in &topo_order {
            // Get the longest path among all dependencies
            let deps = self.depends_on.get(id);
            let best_prefix: Vec<TicketId> = deps
                .map(|deps| {
                    deps.iter()
                        .filter_map(|dep| longest_path.get(dep))
                        .max_by_key(|path| path.len())
                        .cloned()
                        .unwrap_or_default()
                })
                .unwrap_or_default();

            let mut path = best_prefix;
            path.push(id.clone());
            longest_path.insert(id, path);
        }

        // Find the overall longest path
        longest_path
            .into_values()
            .max_by_key(|path| path.len())
            .unwrap_or_default()
    }

    /// Returns the total number of dependency edges in the DAG.
    pub fn edge_count(&self) -> usize {
        self.depends_on.values().map(|deps| deps.len()).sum()
    }

    /// Groups tickets into execution waves (levels of the topological sort).
    ///
    /// Wave 0 contains tickets with no dependencies. Wave N contains tickets
    /// whose maximum dependency wave is N-1. Within each wave, ticket IDs are
    /// sorted alphabetically for deterministic output.
    ///
    /// # Returns
    ///
    /// A vector of waves, where each wave is a vector of ticket IDs.
    /// Returns `Err(DagError::CycleDetected)` if the graph has cycles.
    pub fn execution_waves(&self) -> Result<Vec<Vec<TicketId>>, DagError> {
        let topo_order = self.topological_sort()?;

        let mut wave_of: HashMap<&TicketId, usize> = HashMap::new();
        let mut max_wave: usize = 0;

        for id in &topo_order {
            let deps = self.depends_on.get(id);
            let wave = match deps {
                Some(dep_set) => {
                    dep_set
                        .iter()
                        .map(|dep| wave_of.get(dep).copied().unwrap_or(0))
                        .max()
                        .unwrap_or(0)
                        + 1
                }
                None => 0,
            };
            wave_of.insert(id, wave);
            if wave > max_wave {
                max_wave = wave;
            }
        }

        let mut waves: Vec<Vec<TicketId>> = vec![Vec::new(); max_wave + 1];
        for (id, &wave) in &wave_of {
            waves[wave].push((*id).clone());
        }

        // Sort within each wave for deterministic output
        for wave in &mut waves {
            wave.sort();
        }

        Ok(waves)
    }

    /// Returns statistics about the DAG.
    pub fn stats(&self) -> DagStats {
        let total_tickets = self.nodes.len();
        let done_tickets = self
            .nodes
            .values()
            .filter(|t| t.phase == Phase::Done)
            .count();
        let ready_tickets = self.get_ready_tickets().len();
        let in_progress_tickets = self.get_in_progress_tickets().len();
        let blocked_tickets = total_tickets
            .saturating_sub(done_tickets)
            .saturating_sub(ready_tickets)
            .saturating_sub(in_progress_tickets);
        let critical_path_length = self.critical_path().len();

        DagStats {
            total_tickets,
            done_tickets,
            ready_tickets,
            in_progress_tickets,
            blocked_tickets,
            critical_path_length,
        }
    }
}

impl Default for Dag {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about the DAG state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DagStats {
    /// Total number of tickets in the DAG
    pub total_tickets: usize,
    /// Number of tickets in the "done" phase
    pub done_tickets: usize,
    /// Number of tickets ready to start
    pub ready_tickets: usize,
    /// Number of tickets currently in progress
    pub in_progress_tickets: usize,
    /// Number of tickets blocked by dependencies
    pub blocked_tickets: usize,
    /// Length of the critical path
    pub critical_path_length: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Priority, TicketStatus, TicketType};
    use std::path::PathBuf;

    fn make_ticket(id: &str, phase: Phase, depends_on: Vec<&str>, blocks: Vec<&str>) -> Ticket {
        Ticket {
            id: id.to_string(),
            story: None,
            title: format!("Test ticket {}", id),
            ticket_type: TicketType::Task,
            status: TicketStatus::Open,
            priority: Priority::Medium,
            phase,
            depends_on: depends_on.into_iter().map(|s| s.to_string()).collect(),
            blocks: blocks.into_iter().map(|s| s.to_string()).collect(),
            agent: None,
            model: None,
            file_path: PathBuf::new(),
            content: String::new(),
        }
    }

    #[test]
    fn test_empty_dag() {
        let dag = Dag::from_tickets(vec![]).unwrap();
        assert!(dag.is_empty());
        assert_eq!(dag.len(), 0);
        assert!(dag.get_ready_tickets().is_empty());
    }

    #[test]
    fn test_single_ticket_ready() {
        let ticket = make_ticket("T-001", Phase::Ready, vec![], vec![]);
        let dag = Dag::from_tickets(vec![ticket]).unwrap();

        assert_eq!(dag.len(), 1);
        assert!(dag.can_start(&"T-001".to_string()));

        let ready = dag.get_ready_tickets();
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0], "T-001");
    }

    #[test]
    fn test_done_ticket_not_startable() {
        let ticket = make_ticket("T-001", Phase::Done, vec![], vec![]);
        let dag = Dag::from_tickets(vec![ticket]).unwrap();

        assert!(!dag.can_start(&"T-001".to_string()));
        assert!(dag.get_ready_tickets().is_empty());
    }

    #[test]
    fn test_active_phase_tickets_are_startable() {
        // Ready through Implement are schedulable
        for phase in &[
            Phase::Ready,
            Phase::Research,
            Phase::Design,
            Phase::Structure,
            Phase::Plan,
            Phase::Implement,
        ] {
            let ticket = make_ticket("T-001", *phase, vec![], vec![]);
            let dag = Dag::from_tickets(vec![ticket]).unwrap();
            assert!(
                dag.can_start(&"T-001".to_string()),
                "Phase {:?} should be startable",
                phase
            );
        }
    }

    #[test]
    fn test_review_phase_is_startable() {
        // Review is agent-driven (produces review.md), so it's schedulable
        let ticket = make_ticket("T-001", Phase::Review, vec![], vec![]);
        let dag = Dag::from_tickets(vec![ticket]).unwrap();
        assert!(dag.can_start(&"T-001".to_string()));
    }

    #[test]
    fn test_dependency_chain() {
        let t1 = make_ticket("T-001", Phase::Done, vec![], vec!["T-002"]);
        let t2 = make_ticket("T-002", Phase::Ready, vec!["T-001"], vec!["T-003"]);
        let t3 = make_ticket("T-003", Phase::Ready, vec!["T-002"], vec![]);

        let dag = Dag::from_tickets(vec![t1, t2, t3]).unwrap();

        // T-001 is done, so T-002 should be ready
        assert!(dag.can_start(&"T-002".to_string()));
        // T-002 is not done, so T-003 should not be ready
        assert!(!dag.can_start(&"T-003".to_string()));

        let ready = dag.get_ready_tickets();
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0], "T-002");
    }

    #[test]
    fn test_multiple_dependencies() {
        let t1 = make_ticket("T-001", Phase::Done, vec![], vec![]);
        let t2 = make_ticket("T-002", Phase::Done, vec![], vec![]);
        let t3 = make_ticket("T-003", Phase::Ready, vec!["T-001", "T-002"], vec![]);

        let dag = Dag::from_tickets(vec![t1, t2, t3]).unwrap();

        // Both dependencies are done, so T-003 should be ready
        assert!(dag.can_start(&"T-003".to_string()));
    }

    #[test]
    fn test_multiple_dependencies_partial() {
        let t1 = make_ticket("T-001", Phase::Done, vec![], vec![]);
        let t2 = make_ticket("T-002", Phase::Research, vec![], vec![]);
        let t3 = make_ticket("T-003", Phase::Ready, vec!["T-001", "T-002"], vec![]);

        let dag = Dag::from_tickets(vec![t1, t2, t3]).unwrap();

        // T-002 is not done, so T-003 should not be ready
        assert!(!dag.can_start(&"T-003".to_string()));
    }

    #[test]
    fn test_missing_dependency_error() {
        let t1 = make_ticket("T-001", Phase::Ready, vec!["T-999"], vec![]);

        let result = Dag::from_tickets(vec![t1]);
        assert!(matches!(result, Err(DagError::MissingDependency { .. })));
    }

    #[test]
    fn test_get_blocked_by() {
        let t1 = make_ticket("T-001", Phase::Research, vec![], vec!["T-002", "T-003"]);
        let t2 = make_ticket("T-002", Phase::Ready, vec!["T-001"], vec![]);
        let t3 = make_ticket("T-003", Phase::Ready, vec!["T-001"], vec![]);

        let dag = Dag::from_tickets(vec![t1, t2, t3]).unwrap();

        let blocked = dag.get_blocked_by(&"T-001".to_string());
        assert_eq!(blocked.len(), 2);
        assert!(blocked.contains(&"T-002".to_string()));
        assert!(blocked.contains(&"T-003".to_string()));
    }

    #[test]
    fn test_no_cycle() {
        let t1 = make_ticket("T-001", Phase::Done, vec![], vec!["T-002"]);
        let t2 = make_ticket("T-002", Phase::Ready, vec!["T-001"], vec!["T-003"]);
        let t3 = make_ticket("T-003", Phase::Ready, vec!["T-002"], vec![]);

        let dag = Dag::from_tickets(vec![t1, t2, t3]).unwrap();
        assert_eq!(dag.detect_cycles(), CycleDetectionResult::NoCycle);
    }

    #[test]
    fn test_cycle_detection() {
        // Create a cycle: T-001 -> T-002 -> T-003 -> T-001
        let t1 = make_ticket("T-001", Phase::Ready, vec!["T-003"], vec!["T-002"]);
        let t2 = make_ticket("T-002", Phase::Ready, vec!["T-001"], vec!["T-003"]);
        let t3 = make_ticket("T-003", Phase::Ready, vec!["T-002"], vec!["T-001"]);

        let dag = Dag::from_tickets(vec![t1, t2, t3]).unwrap();
        let result = dag.detect_cycles();

        match result {
            CycleDetectionResult::Cycle(nodes) => {
                assert_eq!(nodes.len(), 3);
            }
            CycleDetectionResult::NoCycle => panic!("Expected cycle to be detected"),
        }
    }

    #[test]
    fn test_topological_sort_linear() {
        let t1 = make_ticket("T-001", Phase::Done, vec![], vec!["T-002"]);
        let t2 = make_ticket("T-002", Phase::Ready, vec!["T-001"], vec!["T-003"]);
        let t3 = make_ticket("T-003", Phase::Ready, vec!["T-002"], vec![]);

        let dag = Dag::from_tickets(vec![t1, t2, t3]).unwrap();
        let sorted = dag.topological_sort().unwrap();

        assert_eq!(sorted.len(), 3);
        // T-001 must come before T-002, T-002 before T-003
        let pos_1 = sorted.iter().position(|id| id == "T-001").unwrap();
        let pos_2 = sorted.iter().position(|id| id == "T-002").unwrap();
        let pos_3 = sorted.iter().position(|id| id == "T-003").unwrap();
        assert!(pos_1 < pos_2);
        assert!(pos_2 < pos_3);
    }

    #[test]
    fn test_topological_sort_diamond() {
        //     T-001
        //     /   \
        //  T-002  T-003
        //     \   /
        //     T-004
        let t1 = make_ticket("T-001", Phase::Done, vec![], vec!["T-002", "T-003"]);
        let t2 = make_ticket("T-002", Phase::Ready, vec!["T-001"], vec!["T-004"]);
        let t3 = make_ticket("T-003", Phase::Ready, vec!["T-001"], vec!["T-004"]);
        let t4 = make_ticket("T-004", Phase::Ready, vec!["T-002", "T-003"], vec![]);

        let dag = Dag::from_tickets(vec![t1, t2, t3, t4]).unwrap();
        let sorted = dag.topological_sort().unwrap();

        assert_eq!(sorted.len(), 4);
        // T-001 must come first, T-004 must come last
        let pos_1 = sorted.iter().position(|id| id == "T-001").unwrap();
        let pos_2 = sorted.iter().position(|id| id == "T-002").unwrap();
        let pos_3 = sorted.iter().position(|id| id == "T-003").unwrap();
        let pos_4 = sorted.iter().position(|id| id == "T-004").unwrap();
        assert!(pos_1 < pos_2);
        assert!(pos_1 < pos_3);
        assert!(pos_2 < pos_4);
        assert!(pos_3 < pos_4);
    }

    #[test]
    fn test_topological_sort_with_cycle() {
        let t1 = make_ticket("T-001", Phase::Ready, vec!["T-002"], vec![]);
        let t2 = make_ticket("T-002", Phase::Ready, vec!["T-001"], vec![]);

        let dag = Dag::from_tickets(vec![t1, t2]).unwrap();
        let result = dag.topological_sort();

        assert!(matches!(result, Err(DagError::CycleDetected(_))));
    }

    #[test]
    fn test_critical_path() {
        // Linear chain: T-001 -> T-002 -> T-003
        let t1 = make_ticket("T-001", Phase::Done, vec![], vec!["T-002"]);
        let t2 = make_ticket("T-002", Phase::Ready, vec!["T-001"], vec!["T-003"]);
        let t3 = make_ticket("T-003", Phase::Ready, vec!["T-002"], vec![]);

        let dag = Dag::from_tickets(vec![t1, t2, t3]).unwrap();
        let path = dag.critical_path();

        assert_eq!(path.len(), 3);
    }

    #[test]
    fn test_stats() {
        let t1 = make_ticket("T-001", Phase::Done, vec![], vec!["T-002"]);
        let t2 = make_ticket("T-002", Phase::Research, vec!["T-001"], vec!["T-003"]);
        let t3 = make_ticket("T-003", Phase::Ready, vec!["T-002"], vec![]);

        let dag = Dag::from_tickets(vec![t1, t2, t3]).unwrap();
        let stats = dag.stats();

        assert_eq!(stats.total_tickets, 3);
        assert_eq!(stats.done_tickets, 1);
        assert_eq!(stats.in_progress_tickets, 1); // T-002 is in Research
        assert_eq!(stats.critical_path_length, 3);
    }

    #[test]
    fn test_concurrent_ready_tickets() {
        // T-001 is done, T-002 and T-003 can run in parallel
        let t1 = make_ticket("T-001", Phase::Done, vec![], vec!["T-002", "T-003"]);
        let t2 = make_ticket("T-002", Phase::Ready, vec!["T-001"], vec![]);
        let t3 = make_ticket("T-003", Phase::Ready, vec!["T-001"], vec![]);

        let dag = Dag::from_tickets(vec![t1, t2, t3]).unwrap();
        let ready = dag.get_ready_tickets();

        assert_eq!(ready.len(), 2);
    }

    #[test]
    fn test_get_ticket() {
        let t1 = make_ticket("T-001", Phase::Ready, vec![], vec![]);
        let dag = Dag::from_tickets(vec![t1]).unwrap();

        let ticket = dag.get_ticket(&"T-001".to_string());
        assert!(ticket.is_some());
        assert_eq!(ticket.unwrap().id, "T-001");

        let missing = dag.get_ticket(&"T-999".to_string());
        assert!(missing.is_none());
    }

    #[test]
    fn test_get_dependencies() {
        let t1 = make_ticket("T-001", Phase::Done, vec![], vec![]);
        let t2 = make_ticket("T-002", Phase::Done, vec![], vec![]);
        let t3 = make_ticket("T-003", Phase::Ready, vec!["T-001", "T-002"], vec![]);

        let dag = Dag::from_tickets(vec![t1, t2, t3]).unwrap();

        let deps = dag.get_dependencies(&"T-003".to_string());
        assert_eq!(deps.len(), 2);
        assert!(deps.contains(&"T-001".to_string()));
        assert!(deps.contains(&"T-002".to_string()));

        let no_deps = dag.get_dependencies(&"T-001".to_string());
        assert!(no_deps.is_empty());
    }

    #[test]
    fn test_tickets_iterator() {
        let t1 = make_ticket("T-001", Phase::Ready, vec![], vec![]);
        let t2 = make_ticket("T-002", Phase::Ready, vec![], vec![]);

        let dag = Dag::from_tickets(vec![t1, t2]).unwrap();

        let ticket_ids: Vec<_> = dag.tickets().map(|t| t.id.clone()).collect();
        assert_eq!(ticket_ids.len(), 2);
        assert!(ticket_ids.contains(&"T-001".to_string()));
        assert!(ticket_ids.contains(&"T-002".to_string()));
    }

    #[test]
    fn test_dag_from_depends_on_only_no_blocks() {
        // Build a DAG where no tickets have blocks fields -- only depends_on.
        // The DAG should correctly compute reverse edges (blocked-by relationships).
        let t1 = make_ticket("T-001", Phase::Done, vec![], vec![]);
        let t2 = make_ticket("T-002", Phase::Ready, vec!["T-001"], vec![]);
        let t3 = make_ticket("T-003", Phase::Ready, vec!["T-001", "T-002"], vec![]);

        let dag = Dag::from_tickets(vec![t1, t2, t3]).unwrap();

        // Verify forward edges (depends_on)
        let deps_t2 = dag.get_dependencies(&"T-002".to_string());
        assert_eq!(deps_t2.len(), 1);
        assert!(deps_t2.contains(&"T-001".to_string()));

        let deps_t3 = dag.get_dependencies(&"T-003".to_string());
        assert_eq!(deps_t3.len(), 2);
        assert!(deps_t3.contains(&"T-001".to_string()));
        assert!(deps_t3.contains(&"T-002".to_string()));

        // Verify reverse edges (blocked-by) are computed from depends_on
        let blocked_by_t1 = dag.get_blocked_by(&"T-001".to_string());
        assert_eq!(blocked_by_t1.len(), 2);
        assert!(blocked_by_t1.contains(&"T-002".to_string()));
        assert!(blocked_by_t1.contains(&"T-003".to_string()));

        let blocked_by_t2 = dag.get_blocked_by(&"T-002".to_string());
        assert_eq!(blocked_by_t2.len(), 1);
        assert!(blocked_by_t2.contains(&"T-003".to_string()));

        let blocked_by_t3 = dag.get_blocked_by(&"T-003".to_string());
        assert!(blocked_by_t3.is_empty());

        // Verify topological sort works
        let sorted = dag.topological_sort().unwrap();
        assert_eq!(sorted.len(), 3);
        let pos_1 = sorted.iter().position(|id| id == "T-001").unwrap();
        let pos_2 = sorted.iter().position(|id| id == "T-002").unwrap();
        let pos_3 = sorted.iter().position(|id| id == "T-003").unwrap();
        assert!(pos_1 < pos_2);
        assert!(pos_2 < pos_3);

        // Verify cycle detection works
        assert_eq!(dag.detect_cycles(), CycleDetectionResult::NoCycle);

        // Verify scheduling: T-001 is done, so T-002 should be ready
        assert!(dag.can_start(&"T-002".to_string()));
        // T-002 is not done, so T-003 should not be ready
        assert!(!dag.can_start(&"T-003".to_string()));
    }

    #[test]
    fn test_runnable_tickets() {
        let t1 = make_ticket("T-001", Phase::Done, vec![], vec!["T-002"]);
        let t2 = make_ticket("T-002", Phase::Ready, vec!["T-001"], vec![]);
        let t3 = make_ticket("T-003", Phase::Research, vec![], vec![]);

        let dag = Dag::from_tickets(vec![t1, t2, t3]).unwrap();
        let mut runnable = dag.get_runnable_tickets();
        runnable.sort_by_key(|t| t.id.clone());

        // T-002 (ready, deps done) and T-003 (research, no deps) are both runnable
        // T-001 is done so it's not runnable
        assert_eq!(runnable.len(), 2);
        assert_eq!(runnable[0].id, "T-002");
        assert_eq!(runnable[1].id, "T-003");
    }

    #[test]
    fn test_end_to_end_scan_to_dag() {
        use crate::ticket::scan_tickets;
        use std::fs;

        let dir = tempfile::tempdir().unwrap();

        // T-001: Done, no deps
        fs::write(
            dir.path().join("T-001.md"),
            "---\nid: T-001\ntitle: first\ntype: task\nstatus: open\npriority: high\nphase: done\n---\n\nDone ticket.\n",
        ).unwrap();

        // T-002: Ready, depends on T-001
        fs::write(
            dir.path().join("T-002.md"),
            "---\nid: T-002\ntitle: second\ntype: task\nstatus: open\npriority: medium\nphase: ready\ndepends_on: [T-001]\n---\n\nReady ticket.\n",
        ).unwrap();

        // T-003: Ready, depends on T-001 and T-002
        fs::write(
            dir.path().join("T-003.md"),
            "---\nid: T-003\ntitle: third\ntype: task\nstatus: open\npriority: low\nphase: ready\ndepends_on: [T-001, T-002]\n---\n\nBlocked ticket.\n",
        ).unwrap();

        // Parse tickets from disk
        let tickets = scan_tickets(dir.path()).unwrap();
        assert_eq!(tickets.len(), 3);

        // Build DAG
        let dag = Dag::from_tickets(tickets).unwrap();
        assert_eq!(dag.len(), 3);

        // T-001 is done, so T-002 should be ready (its only dep is done)
        let ready = dag.get_ready_tickets();
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0], "T-002");

        // T-003 depends on T-002 which is not done, so it should NOT be ready
        assert!(!dag.can_start(&"T-003".to_string()));

        // Verify dependency edges
        let deps_t3 = dag.get_dependencies(&"T-003".to_string());
        assert_eq!(deps_t3.len(), 2);
        assert!(deps_t3.contains(&"T-001".to_string()));
        assert!(deps_t3.contains(&"T-002".to_string()));

        // Topological sort should have T-001 first
        let sorted = dag.topological_sort().unwrap();
        let pos_1 = sorted.iter().position(|id| id == "T-001").unwrap();
        let pos_2 = sorted.iter().position(|id| id == "T-002").unwrap();
        let pos_3 = sorted.iter().position(|id| id == "T-003").unwrap();
        assert!(pos_1 < pos_2);
        assert!(pos_2 < pos_3);

        // No cycles
        assert_eq!(dag.detect_cycles(), CycleDetectionResult::NoCycle);
    }

    #[test]
    fn test_edge_count_empty() {
        let dag = Dag::from_tickets(vec![]).unwrap();
        assert_eq!(dag.edge_count(), 0);
    }

    #[test]
    fn test_edge_count_chain() {
        let t1 = make_ticket("T-001", Phase::Done, vec![], vec![]);
        let t2 = make_ticket("T-002", Phase::Ready, vec!["T-001"], vec![]);
        let t3 = make_ticket("T-003", Phase::Ready, vec!["T-002"], vec![]);
        let dag = Dag::from_tickets(vec![t1, t2, t3]).unwrap();
        assert_eq!(dag.edge_count(), 2);
    }

    #[test]
    fn test_edge_count_diamond() {
        let t1 = make_ticket("T-001", Phase::Done, vec![], vec![]);
        let t2 = make_ticket("T-002", Phase::Ready, vec!["T-001"], vec![]);
        let t3 = make_ticket("T-003", Phase::Ready, vec!["T-001"], vec![]);
        let t4 = make_ticket("T-004", Phase::Ready, vec!["T-002", "T-003"], vec![]);
        let dag = Dag::from_tickets(vec![t1, t2, t3, t4]).unwrap();
        assert_eq!(dag.edge_count(), 4);
    }

    #[test]
    fn test_execution_waves_no_deps() {
        let t1 = make_ticket("T-001", Phase::Ready, vec![], vec![]);
        let t2 = make_ticket("T-002", Phase::Ready, vec![], vec![]);
        let dag = Dag::from_tickets(vec![t1, t2]).unwrap();
        let waves = dag.execution_waves().unwrap();
        assert_eq!(waves.len(), 1);
        assert_eq!(waves[0], vec!["T-001", "T-002"]);
    }

    #[test]
    fn test_execution_waves_chain() {
        let t1 = make_ticket("T-001", Phase::Done, vec![], vec![]);
        let t2 = make_ticket("T-002", Phase::Ready, vec!["T-001"], vec![]);
        let t3 = make_ticket("T-003", Phase::Ready, vec!["T-002"], vec![]);
        let dag = Dag::from_tickets(vec![t1, t2, t3]).unwrap();
        let waves = dag.execution_waves().unwrap();
        assert_eq!(waves.len(), 3);
        assert_eq!(waves[0], vec!["T-001"]);
        assert_eq!(waves[1], vec!["T-002"]);
        assert_eq!(waves[2], vec!["T-003"]);
    }

    #[test]
    fn test_execution_waves_diamond() {
        let t1 = make_ticket("T-001", Phase::Done, vec![], vec![]);
        let t2 = make_ticket("T-002", Phase::Ready, vec!["T-001"], vec![]);
        let t3 = make_ticket("T-003", Phase::Ready, vec!["T-001"], vec![]);
        let t4 = make_ticket("T-004", Phase::Ready, vec!["T-002", "T-003"], vec![]);
        let dag = Dag::from_tickets(vec![t1, t2, t3, t4]).unwrap();
        let waves = dag.execution_waves().unwrap();
        assert_eq!(waves.len(), 3);
        assert_eq!(waves[0], vec!["T-001"]);
        assert_eq!(waves[1], vec!["T-002", "T-003"]);
        assert_eq!(waves[2], vec!["T-004"]);
    }

    #[test]
    fn test_execution_waves_cycle_error() {
        let t1 = make_ticket("T-001", Phase::Ready, vec!["T-002"], vec![]);
        let t2 = make_ticket("T-002", Phase::Ready, vec!["T-001"], vec![]);
        let dag = Dag::from_tickets(vec![t1, t2]).unwrap();
        assert!(matches!(
            dag.execution_waves(),
            Err(DagError::CycleDetected(_))
        ));
    }

    #[test]
    fn test_blocked_status_prevents_scheduling() {
        let mut t1 = make_ticket("T-001", Phase::Ready, vec![], vec![]);
        t1.status = TicketStatus::Blocked;

        let dag = Dag::from_tickets(vec![t1]).unwrap();

        assert!(!dag.can_start(&"T-001".to_string()));
        assert!(dag.get_ready_tickets().is_empty());
    }

    #[test]
    fn test_blocked_status_with_deps_done() {
        let t1 = make_ticket("T-001", Phase::Done, vec![], vec![]);
        let mut t2 = make_ticket("T-002", Phase::Ready, vec!["T-001"], vec![]);
        t2.status = TicketStatus::Blocked;

        let dag = Dag::from_tickets(vec![t1, t2]).unwrap();

        // Even though T-001 is done, T-002 is explicitly blocked
        assert!(!dag.can_start(&"T-002".to_string()));
        assert!(dag.get_ready_tickets().is_empty());
    }

    // --- all_ancestors_done tests ---

    #[test]
    fn test_all_ancestors_done_no_deps() {
        let t1 = make_ticket("T-001", Phase::Ready, vec![], vec![]);
        let dag = Dag::from_tickets(vec![t1]).unwrap();
        assert!(dag.all_ancestors_done(&"T-001".to_string()));
    }

    #[test]
    fn test_all_ancestors_done_direct_dep_done() {
        let t1 = make_ticket("T-001", Phase::Done, vec![], vec![]);
        let t2 = make_ticket("T-002", Phase::Ready, vec!["T-001"], vec![]);
        let dag = Dag::from_tickets(vec![t1, t2]).unwrap();
        assert!(dag.all_ancestors_done(&"T-002".to_string()));
    }

    #[test]
    fn test_all_ancestors_done_direct_dep_not_done() {
        let t1 = make_ticket("T-001", Phase::Research, vec![], vec![]);
        let t2 = make_ticket("T-002", Phase::Ready, vec!["T-001"], vec![]);
        let dag = Dag::from_tickets(vec![t1, t2]).unwrap();
        assert!(!dag.all_ancestors_done(&"T-002".to_string()));
    }

    #[test]
    fn test_all_ancestors_done_transitive_gap() {
        // A(open) → B(done) → C(ready): C should NOT be schedulable
        let a = make_ticket("A", Phase::Research, vec![], vec![]);
        let b = make_ticket("B", Phase::Done, vec!["A"], vec![]);
        let c = make_ticket("C", Phase::Ready, vec!["B"], vec![]);
        let dag = Dag::from_tickets(vec![a, b, c]).unwrap();
        assert!(!dag.all_ancestors_done(&"C".to_string()));
    }

    #[test]
    fn test_all_ancestors_done_full_chain() {
        // A(done) → B(done) → C(ready): C is schedulable
        let a = make_ticket("A", Phase::Done, vec![], vec![]);
        let b = make_ticket("B", Phase::Done, vec!["A"], vec![]);
        let c = make_ticket("C", Phase::Ready, vec!["B"], vec![]);
        let dag = Dag::from_tickets(vec![a, b, c]).unwrap();
        assert!(dag.all_ancestors_done(&"C".to_string()));
    }

    #[test]
    fn test_all_ancestors_done_diamond_partial() {
        //     A(done)
        //    /       \
        // B(done)  C(open)
        //    \       /
        //     D(ready)
        let a = make_ticket("A", Phase::Done, vec![], vec![]);
        let b = make_ticket("B", Phase::Done, vec!["A"], vec![]);
        let c = make_ticket("C", Phase::Research, vec!["A"], vec![]);
        let d = make_ticket("D", Phase::Ready, vec!["B", "C"], vec![]);
        let dag = Dag::from_tickets(vec![a, b, c, d]).unwrap();
        // C is not done, so D should not be schedulable
        assert!(!dag.all_ancestors_done(&"D".to_string()));
    }

    #[test]
    fn test_all_ancestors_done_diamond_full() {
        //     A(done)
        //    /       \
        // B(done)  C(done)
        //    \       /
        //     D(ready)
        let a = make_ticket("A", Phase::Done, vec![], vec![]);
        let b = make_ticket("B", Phase::Done, vec!["A"], vec![]);
        let c = make_ticket("C", Phase::Done, vec!["A"], vec![]);
        let d = make_ticket("D", Phase::Ready, vec!["B", "C"], vec![]);
        let dag = Dag::from_tickets(vec![a, b, c, d]).unwrap();
        assert!(dag.all_ancestors_done(&"D".to_string()));
    }

    #[test]
    fn test_can_start_requires_transitive_ancestors() {
        // The original regression case:
        // A(open) → B(done) → C(ready)
        // C's direct dep B is done, but B's dep A is not.
        // can_start should return false for C.
        let a = make_ticket("A", Phase::Research, vec![], vec![]);
        let b = make_ticket("B", Phase::Done, vec!["A"], vec![]);
        let c = make_ticket("C", Phase::Ready, vec!["B"], vec![]);
        let dag = Dag::from_tickets(vec![a, b, c]).unwrap();

        assert!(!dag.can_start(&"C".to_string()));
        // A is still runnable (Research, no deps), but C must not appear
        let ready = dag.get_ready_tickets();
        assert!(!ready.contains(&"C".to_string()));
    }
}
