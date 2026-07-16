use std::path::PathBuf;

const WASM_HEADER: &[u8] = b"\0asm\x01\0\0\0";

fn main() {
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let require_embedded_wasm = std::env::var("LISA_REQUIRE_EMBEDDED_WASM").as_deref() == Ok("1");
    println!("cargo:rerun-if-env-changed=LISA_REQUIRE_EMBEDDED_WASM");

    // Find the workspace root by walking up from CARGO_MANIFEST_DIR
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let workspace_root = manifest_dir
        .parent() // crates/
        .and_then(|p| p.parent()) // workspace root
        .expect("Could not find workspace root");

    let wasm_source = workspace_root.join("target/wasm32-wasip1/release/lisa.wasm");
    let wasm_dest = out_dir.join("lisa.wasm");

    // Tell Cargo to rebuild if the WASM file changes
    println!("cargo:rerun-if-changed={}", wasm_source.display());

    match std::fs::read(&wasm_source) {
        Ok(wasm) => {
            assert!(
                wasm.starts_with(WASM_HEADER),
                "Release WASM at {} is empty or does not have a valid WebAssembly header",
                wasm_source.display()
            );
            std::fs::write(&wasm_dest, wasm).expect("Failed to copy lisa.wasm to OUT_DIR");
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound && !require_embedded_wasm => {
            // Preserve the placeholder for CLI-only developer builds. Release CI
            // sets LISA_REQUIRE_EMBEDDED_WASM after building and checking the plugin.
            std::fs::write(&wasm_dest, b"").expect("Failed to write placeholder lisa.wasm");
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            panic!(
                "Release WASM is required but missing at {}",
                wasm_source.display()
            );
        }
        Err(error) => panic!(
            "Failed to read release WASM at {}: {error}",
            wasm_source.display()
        ),
    }
}
