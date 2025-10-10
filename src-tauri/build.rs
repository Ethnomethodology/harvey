// src-tauri/build.rs
use std::{env, fs, path::{PathBuf}, process::Command};
fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());

    // --- FFmpeg handling for macOS, Linux, and Windows ---
    if cfg!(target_os = "macos") || cfg!(target_os = "linux") || cfg!(target_os = "windows") {
        let ffmpeg_built_path = manifest_dir.join("binaries").join("ffmpeg");
        let profile = env::var("PROFILE").unwrap();

        // We build ffmpeg if:
        // 1. It's a release build (e.g., `tauri build`).
        // 2. It's a debug build AND the ffmpeg libraries don't already exist.
        if profile == "release" || !ffmpeg_built_path.exists() {
            let reason = if profile == "release" {
                "release build"
            } else {
                "dev build and libraries are missing"
            };
            println!("cargo:info=Building FFmpeg from source (reason: {})...", reason);

            let script_path = manifest_dir.join("scripts").join("build-ffmpeg.sh");
            println!("cargo:rerun-if-changed={}", script_path.to_str().unwrap());

            let command = if cfg!(target_os = "windows") { "bash" } else { "sh" };
            let status = Command::new(command)
                .arg(&script_path)
                .status()
                .expect("Failed to execute build-ffmpeg.sh script");

            if !status.success() {
                panic!("ffmpeg build script failed with exit code: {}", status);
            }
        } else {
            println!("cargo:info=Skipping FFmpeg build: libraries already exist for dev build.");
        }
    }

    // --- Python Bundling ---
    let python_bundle_path = manifest_dir.join("python");
    if cfg!(target_os = "macos") || cfg!(target_os = "linux") || cfg!(target_os = "windows") {
        let profile = env::var("PROFILE").unwrap();
        // We bundle Python if:
        // 1. It's a release build.
        // 2. It's a debug build AND the python directory doesn't already exist.
        if profile == "release" || !python_bundle_path.exists() {
            let reason = if profile == "release" {
                "release build"
            } else {
                "dev build and python bundle is missing"
            };
            println!("cargo:info=Bundling self-contained Python (reason: {})...", reason);

            let script_path = manifest_dir.join("scripts").join("bundle-python.sh");
            println!("cargo:rerun-if-changed={}", script_path.to_str().unwrap());

            let command = if cfg!(target_os = "windows") { "bash" } else { "sh" };
            let status = Command::new(command)
                .arg(&script_path)
                .status()
                .expect("Failed to execute bundle-python.sh script");

            if !status.success() {
                panic!("Python bundling script failed with exit code: {}", status);
            }
        } else {
            println!("cargo:info=Skipping Python bundling: python directory already exists for dev build.");
        }
    }

    // --- whisper.cpp handling for macOS, Linux, and Windows ---
    if cfg!(target_os = "macos") || cfg!(target_os = "linux") || cfg!(target_os = "windows") {
        let whisper_built_path = manifest_dir.join("binaries").join("whisper.cpp");
        let profile = env::var("PROFILE").unwrap();

        // We build whisper.cpp if:
        // 1. It's a release build (e.g., `tauri build`).
        // 2. It's a debug build AND the whisper.cpp binaries don't already exist.
        if profile == "release" || !whisper_built_path.exists() {
            let reason = if profile == "release" {
                "release build"
            } else {
                "dev build and binaries are missing"
            };
            println!("cargo:info=Building whisper.cpp from source (reason: {})...", reason);

            let script_path = manifest_dir.join("scripts").join("build-whisper.sh");
            println!("cargo:rerun-if-changed={}", script_path.to_str().unwrap());

            let command = if cfg!(target_os = "windows") { "bash" } else { "sh" };
            let status = Command::new(command)
                .arg(&script_path)
                .status()
                .expect("Failed to execute build-whisper.sh script");

            if !status.success() {
                panic!("whisper.cpp build script failed with exit code: {}", status);
            }
        } else {
            println!("cargo:info=Skipping whisper.cpp build: binaries already exist for dev build.");
        }
    }

    // --- Copy Python scripts to target/debug/scripts for development ---
    let scripts_source_dir = manifest_dir.join("scripts");
    let scripts_target_dir = manifest_dir.join("target").join(env::var("PROFILE").unwrap()).join("scripts");
    fs::create_dir_all(&scripts_target_dir).expect("Failed to create target scripts directory");

    let python_scripts_to_copy = vec![
        "convert_with_pandoc.py",
        // Add other Python scripts that are directly invoked by Rust code if any
    ];

    for script_name in python_scripts_to_copy {
        let source_path = scripts_source_dir.join(script_name);
        let dest_path = scripts_target_dir.join(script_name);
        if source_path.exists() {
            fs::copy(&source_path, &dest_path).expect(&format!("Failed to copy {} to {}", source_path.display(), dest_path.display()));
            println!("cargo:info=Copied {} to {}", source_path.display(), dest_path.display());
        } else {
            println!("cargo:warning=Python script {} not found at {}", script_name, source_path.display());
        }
    }

    // Rerun this build script if build.rs itself changes
    println!("cargo:rerun-if-changed=build.rs");
    tauri_build::build();
}
