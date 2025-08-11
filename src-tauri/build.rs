// src-tauri/build.rs
use std::{env, fs, io, path::{Path, PathBuf}, process::Command};
use sha2::{Digest, Sha256};
use hex;

/// Finds a file by name in a directory and its subdirectories.
fn find_file_in_dir(dir: &Path, file_name: &str) -> Option<PathBuf> {
    if !dir.is_dir() {
        return None;
    }
    for entry in fs::read_dir(dir).ok()? {
        let entry = entry.ok()?;
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = find_file_in_dir(&path, file_name) {
                return Some(found);
            }
        } else if path.file_name().map_or(false, |name| name == file_name) {
            return Some(path);
        }
    }
    None
}

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

    // --- Binaries from harvey-sidecars repo ---
    let harvey_repo = "dipanjan92/harvey-sidecars";
    let harvey_tag = "v0.2.0";
    let harvey_binaries = ["diarize-cli", "ffmpeg", "whisper-cli", "whisper-stream"];

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

    // --- Pandoc from official repo ---
    let pandoc_final_name = format!("pandoc-{}{}", target, extension);
    let pandoc_dest_path = sidecar_dir.join(&pandoc_final_name);
    if !pandoc_dest_path.exists() {
        let (pandoc_os, pandoc_arch) = match target.as_str() {
            "aarch64-apple-darwin" => ("macOS", "arm64-"),
            "x86_64-apple-darwin" => ("macOS", "x86_64-"),
            "x86_64-pc-windows-msvc" => ("windows-x86_64", ""),
            "aarch64-pc-windows-msvc" => ("windows-arm64", ""), // Assuming pandoc supports this
            "x86_64-unknown-linux-gnu" => ("linux-x86_64", ""), // Assuming pandoc naming
            "aarch64-unknown-linux-gnu" => ("linux-arm64", ""), // Assuming pandoc naming
            _ => ("", ""),
        };

        if !pandoc_os.is_empty() {
            let pandoc_release_url = "https://api.github.com/repos/jgm/pandoc/releases/latest";
            let pandoc_release_json = Command::new("curl").args(&["-sL", pandoc_release_url]).output().expect("Failed to fetch Pandoc release info");
            let pandoc_release_str = String::from_utf8_lossy(&pandoc_release_json.stdout);
            let pandoc_tag = pandoc_release_str
                .lines()
                .find(|line| line.contains("\"tag_name\""))
                .and_then(|line| line.split(':').nth(1))
                .map(|value| value.trim().trim_matches(|c| c == '"' || c == ','))
                .unwrap_or("3.2.1"); // Fallback tag

            let pandoc_asset_name = format!("pandoc-{}-{}{}.zip", pandoc_tag, pandoc_arch, pandoc_os);
            let pandoc_url = format!("https://github.com/jgm/pandoc/releases/download/{}/{}", pandoc_tag, pandoc_asset_name);
            let archive_path = temp_dir.join(&pandoc_asset_name);

            download_file(&pandoc_url, &archive_path);

            println!("cargo:info=Extracting {}...", archive_path.display());
            let status = Command::new("unzip")
                .args(&["-o", &archive_path.to_string_lossy(), "-d", &temp_dir.to_string_lossy()])
                .status()
                .expect("Failed to start unzip command");
            if !status.success() {
                panic!("Failed to extract pandoc archive: {}. Unzip exit code: {}", archive_path.display(), status);
            }

            let search_filename = format!("pandoc{}", extension);
            if let Some(found_path) = find_file_in_dir(&temp_dir, &search_filename) {
                fs::rename(&found_path, &pandoc_dest_path).expect("Failed to move pandoc binary");
            } else {
                println!("cargo:warning=Could not find pandoc in the extracted archive.");
            }
        }
    }
    
    if !target.contains("windows") {
        if pandoc_dest_path.exists() {
            Command::new("chmod").arg("+x").arg(&pandoc_dest_path).status().expect("Failed to make pandoc executable");
        }
    }


    // Clean up
    fs::remove_dir_all(&temp_dir).expect("Failed to remove temp directory");

    // Rerun this build script if build.rs itself changes
    println!("cargo:rerun-if-changed=build.rs");
    tauri_build::build();
}
