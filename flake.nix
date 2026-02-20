{
  description = "Lisa — DAG-driven concurrent task scheduling for Zellij";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    crane.url = "github:ipetkov/crane";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, crane, rust-overlay }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];
      forEachSystem = nixpkgs.lib.genAttrs systems;

      # Per-system helpers, shared across packages/checks/devShells
      perSystem = system:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [ rust-overlay.overlays.default ];
          };

          # overrideToolchain takes a function since crane v0.18
          craneLib = (crane.mkLib pkgs).overrideToolchain (p:
            p.rust-bin.stable.latest.default.override {
              extensions = [ "rust-src" "clippy" "rustfmt" ];
              targets = [ "wasm32-wasip1" ];
            }
          );

          # Include Rust sources plus .md files needed by include_str!
          # Use ./. as base (not cleanCargoSource) so .md files aren't
          # stripped before the custom filter runs.
          src = pkgs.lib.cleanSourceWith {
            src = ./.;
            filter = path: type:
              (craneLib.filterCargoSources path type)
              || (builtins.match ".*\\.md$" path != null);
          };

          commonArgs = {
            inherit src;
            pname = "lisa";
            version = "0.1.6";
            strictDeps = true;
          };

          cargoArtifacts = craneLib.buildDepsOnly commonArgs;
        in
        { inherit pkgs craneLib src commonArgs cargoArtifacts; };
    in
    {
      packages = forEachSystem (system:
        let
          s = perSystem system;
        in
        {
          default = s.craneLib.buildPackage (s.commonArgs // {
            inherit (s) cargoArtifacts;

            nativeBuildInputs = [ s.pkgs.makeWrapper ];

            # Stage 1: build WASM plugin before the CLI build picks it up
            preBuild = ''
              cargo build -p lisa-plugin --target wasm32-wasip1 --release
            '';

            # Stage 2: build only the CLI (which embeds the WASM via build.rs)
            cargoBuildCommand = "cargo build -p lisa-cli --profile release";

            # Tests run on native target
            cargoTestCommand = "cargo test --workspace";

            # Wrap the binary so zellij is on PATH at runtime
            postInstall = ''
              wrapProgram $out/bin/lisa \
                --prefix PATH : ${s.pkgs.zellij}/bin
            '';
          });
        }
      );

      checks = forEachSystem (system:
        let
          s = perSystem system;
        in
        {
          # The package build itself is a check
          lisa = self.packages.${system}.default;

          lisa-clippy = s.craneLib.cargoClippy (s.commonArgs // {
            inherit (s) cargoArtifacts;
            cargoClippyExtraArgs = "--workspace -- -D warnings";
          });

          lisa-fmt = s.craneLib.cargoFmt {
            inherit (s) src;
            pname = "lisa";
            version = "0.1.6";
          };
        }
      );

      apps = forEachSystem (system: {
        default = {
          type = "app";
          program = "${self.packages.${system}.default}/bin/lisa";
        };
      });

      devShells = forEachSystem (system:
        let
          s = perSystem system;
        in
        {
          default = s.pkgs.mkShell {
            nativeBuildInputs = [
              (s.pkgs.rust-bin.stable.latest.default.override {
                extensions = [ "rust-src" "clippy" "rustfmt" ];
                targets = [ "wasm32-wasip1" ];
              })
              s.pkgs.just
              s.pkgs.zellij
              s.pkgs.cargo-watch
            ];
          };
        }
      );
    };
}
