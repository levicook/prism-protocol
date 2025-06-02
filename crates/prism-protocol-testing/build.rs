use std::{env, path::PathBuf, process::Command};

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR")
        .expect("CARGO_MANIFEST_DIR not set");
    
    let manifest_path = PathBuf::from(&manifest_dir);
    let workspace_root = manifest_path
        .parent()  // crates/
        .unwrap()
        .parent()  // workspace root
        .unwrap();
    
    let program_manifest = workspace_root
        .join("programs")
        .join("prism-protocol")
        .join("Cargo.toml");
    
    let program_src = workspace_root
        .join("programs")
        .join("prism-protocol")
        .join("src");
    
    // Tell cargo to rerun this build script if the program source changes
    println!("cargo:rerun-if-changed={}", program_manifest.display());
    println!("cargo:rerun-if-changed={}", program_src.display());
    
    // Build the Prism Protocol program
    let output = Command::new("cargo")
        .args(&["build-sbf", "--manifest-path", &program_manifest.to_string_lossy()])
        .output();
    
    match output {
        Ok(output) => {
            if !output.status.success() {
                eprintln!("Failed to build Prism Protocol program:");
                eprintln!("stdout: {}", String::from_utf8_lossy(&output.stdout));
                eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));
                std::process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("Failed to execute cargo build-sbf: {}", e);
            eprintln!("Make sure you have the Solana CLI tools installed and in your PATH");
            std::process::exit(1);
        }
    }
} 