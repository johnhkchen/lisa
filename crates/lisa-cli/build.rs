use std::path::PathBuf;

fn main() {
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());

    // Find the workspace root by walking up from CARGO_MANIFEST_DIR
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let workspace_root = manifest_dir
        .parent() // crates/
        .and_then(|p| p.parent()) // workspace root
        .expect("Could not find workspace root");

    let wasm_source = workspace_root.join("target/wasm32-wasip1/release/lisa.wasm");
    let wasm_dest = out_dir.join("lisa.wasm");

    // Tell Cargo to rebuild if the WASM file changes
    println!(
        "cargo:rerun-if-changed={}",
        wasm_source.display()
    );

    if wasm_source.exists() {
        std::fs::copy(&wasm_source, &wasm_dest).expect("Failed to copy lisa.wasm to OUT_DIR");
    } else {
        // Write empty placeholder for dev builds where the plugin isn't compiled yet
        std::fs::write(&wasm_dest, b"").expect("Failed to write placeholder lisa.wasm");
    }
}
