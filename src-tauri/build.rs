// src-tauri/build.rs
use std::{env, fs, path::PathBuf};

fn main() {
    // Get the output directory (e.g., target/debug/build/harvey-1-abcdef/out)
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    // Determine the profile ("debug" or "release") based on opt-level
    let profile = env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());

    // Find the root target directory (e.g., target/debug or target/release)
    // This usually is three levels up from OUT_DIR for crate builds.
    let target_dir = out_dir
        .ancestors()
        .nth(3)
        .expect("Failed to determine target directory from OUT_DIR")
        .to_path_buf();

    println!("cargo:info=Build script detected target directory: {:?}", target_dir);
    println!("cargo:info=Build profile: {}", profile);


    // Source directory of the models binaries (relative to Cargo.toml)
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let models_src_dir = manifest_dir.join("models");
    println!("cargo:info=models source directory: {:?}", models_src_dir);


    // Destination directory *within* the target directory
    let models_dest_dir = target_dir.join("models");
     println!("cargo:info=models destination directory: {:?}", models_dest_dir);


    // --- Crucial: Rerun instructions for Cargo ---
    // Rerun this build script if build.rs itself changes
    println!("cargo:rerun-if-changed=build.rs");
    // Rerun this build script if the contents of the models source directory change
    println!("cargo:rerun-if-changed={}", models_src_dir.display());


    // --- Perform the Copy ---
    if models_src_dir.exists() && models_src_dir.is_dir() {
        // Ensure destination directory exists
        if !models_dest_dir.exists() {
            println!("cargo:info=Creating destination models directory: {:?}", models_dest_dir);
            fs::create_dir_all(&models_dest_dir)
                .expect("Failed to create models destination directory in target");
        }

        // Copy each file from source to destination
        match fs::read_dir(&models_src_dir) {
            Ok(entries) => {
                println!("cargo:info=Copying models files...");
                for entry in entries {
                    if let Ok(entry) = entry {
                        let src_path = entry.path();
                        if src_path.is_file() {
                            let dest_path = models_dest_dir.join(entry.file_name());
                            println!("cargo:info=  Copying {:?} -> {:?}", src_path.file_name().unwrap_or_default(), dest_path);
                            match fs::copy(&src_path, &dest_path) {
                                Ok(_) => {} // File copied successfully
                                Err(e) => {
                                    // Use cargo:warning for non-fatal issues during build
                                    eprintln!("cargo:warning=Failed to copy models file {:?} to {:?}: {}", src_path, dest_path, e);
                                }
                            }
                        }
                    }
                }
                println!("cargo:info=Finished copying models files.");
            }
            Err(e) => {
                eprintln!("cargo:warning=Failed to read models source directory {:?}: {}", models_src_dir, e);
            }
        }
    } else {
         eprintln!("cargo:warning=models source directory not found or is not a directory: {:?}", models_src_dir);
    }

    // Finally, let tauri-build do its thing (important!)
    tauri_build::build();
}