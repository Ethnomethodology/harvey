// src-tauri/build.rs
use std::{env, fs, io, path::{Path, PathBuf}, process::Command};
use sha2::{Digest, Sha256};
use hex;

fn download_file(url: &str, dest_path: &Path) {
    println!("cargo:info=Downloading from {} to {}...", url, dest_path.display());
    let status = Command::new("curl")
        .args(&["-fL", url, "-o", &dest_path.to_string_lossy()])
        .status()
        .expect("Failed to start curl command");

    if !status.success() {
        panic!("Failed to download file from {}. Curl exit code: {}", url, status);
    }
}

fn verify_sha256(path: &Path, expected_hash: &str) {
    let mut file = fs::File::open(path).expect("Failed to open file for hashing");
    let mut sha256 = Sha256::new();
    io::copy(&mut file, &mut sha256).expect("Failed to read file for hashing");
    let hash = sha256.finalize();
    let hex_hash = hex::encode(hash);

    if hex_hash != expected_hash {
        panic!("SHA256 mismatch for {}. Expected: {}, Got: {}", path.display(), expected_hash, hex_hash);
    }
    println!("cargo:info=SHA256 verified for {}", path.display());
}

fn main() {
    let target = env::var("TARGET").unwrap();
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let sidecar_dir = manifest_dir.join("sidecar");
    fs::create_dir_all(&sidecar_dir).expect("Failed to create sidecar directory");

    let temp_dir = manifest_dir.join("temp_sidecar_downloads");
    fs::create_dir_all(&temp_dir).expect("Failed to create temp directory");

    let (os_suffix, extension) = match target.as_str() {
        "aarch64-apple-darwin" => ("macos-arm64", ""),
        "x86_64-apple-darwin" => ("macos-x86_64", ""),
        "aarch64-pc-windows-msvc" => ("windows-arm64", ".exe"),
        "x86_64-pc-windows-msvc" => ("windows-x86_64", ".exe"),
        "aarch64-unknown-linux-gnu" => ("linux-arm64", ""),
        "x86_64-unknown-linux-gnu" => ("linux-x86_64", ""),
        _ => {
            println!("cargo:warning=Unsupported target for sidecar download: {}. Skipping.", target);
            tauri_build::build();
            return;
        }
    };

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

    // --- Binaries from harvey-sidecars repo ---
    let harvey_repo = "dipanjan92/harvey-sidecars";
    let harvey_tag = "v0.2.0";
    // Exclude ffmpeg if we are building it from source
    let harvey_binaries = if cfg!(target_os = "macos") || cfg!(target_os = "linux") || cfg!(target_os = "windows") {
        vec![]
    } else {
        vec!["ffmpeg"]
    };

    for binary_name in &harvey_binaries {
        let final_binary_name = format!("{}-{}{}", binary_name, target, extension);
        let dest_path = sidecar_dir.join(&final_binary_name);

        if !dest_path.exists() {
            let asset_name = format!("{}-{}{}", binary_name, os_suffix, extension);
            let temp_path = temp_dir.join(&asset_name);
            let url = format!("https://github.com/{}/releases/download/{}/{}", harvey_repo, harvey_tag, asset_name);
            
            download_file(&url, &temp_path);
            
            let sha_asset_name = format!("{}.sha256", asset_name);
            let sha_url = format!("https://github.com/{}/releases/download/{}/{}", harvey_repo, harvey_tag, sha_asset_name);
            let sha_temp_path = temp_dir.join(&sha_asset_name);
            download_file(&sha_url, &sha_temp_path);

            let expected_hash_content = fs::read_to_string(&sha_temp_path).expect("Failed to read sha256 file");
            let expected_hash = expected_hash_content.split_whitespace().next().unwrap_or("").trim();

            if !expected_hash.is_empty() {
                verify_sha256(&temp_path, expected_hash);
            } else {
                println!("cargo:warning=Could not read SHA256 hash from {}. Skipping verification.", sha_temp_path.display());
            }
            
            fs::remove_file(&sha_temp_path).expect("Failed to remove sha256 file");

            fs::rename(&temp_path, &dest_path).expect("Failed to move downloaded binary");
        }

        if !target.contains("windows") {
            Command::new("chmod").arg("+x").arg(&dest_path).status().expect("Failed to make binary executable");
        }
    }

    // Clean up
    fs::remove_dir_all(&temp_dir).expect("Failed to remove temp directory");

    // Rerun this build script if build.rs itself changes
    println!("cargo:rerun-if-changed=build.rs");
    tauri_build::build();
}
