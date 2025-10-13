// src-tauri/build.rs

use std::env;
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::Command;
use zip::ZipArchive;
use anyhow::{Context, Result};

const SIDECAR_VERSION: &str = "v0.2.0";
const SIDECARS_BASE_URL: &str = "https://github.com/dipanjan92/harvey-sidecars/releases/download";

fn main() -> Result<()> {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let sidecars_dir = manifest_dir.join("sidecars");
    let profile = env::var("PROFILE").unwrap();

    if profile == "release" || !sidecars_dir.exists() {
        let reason = if profile == "release" { "release build" } else { "dev build and sidecars are missing" };
        println!("cargo:info=Downloading pre-compiled sidecars (reason: {})...", reason);

        let target_platform = get_target_platform_string()?;
        println!("cargo:info=Detected target platform: {}", target_platform);

        if sidecars_dir.exists() {
            fs::remove_dir_all(&sidecars_dir)?;
        }
        fs::create_dir_all(&sidecars_dir)?;

        download_and_unzip("ffmpeg", &target_platform, &sidecars_dir)?;
        download_and_unzip("whisper-sidecars", &target_platform, &sidecars_dir)?;

        rename_whisper_binaries_for_tauri(&sidecars_dir)?;

        println!("cargo:info=Sidecars downloaded and extracted successfully.");
    } else {
        println!("cargo:info=Skipping sidecar download: sidecars directory already exists for dev build.");
    }

    // --- Python Bundling (existing logic) ---
    let python_bundle_path = manifest_dir.join("python");
    if cfg!(target_os = "macos") || cfg!(target_os = "linux") || cfg!(target_os = "windows") {
        if profile == "release" || !python_bundle_path.exists() {
            let reason = if profile == "release" { "release build" } else { "dev build and python bundle is missing" };
            println!("cargo:info=Bundling self-contained Python (reason: {})...", reason);
            let script_path = manifest_dir.join("scripts").join("bundle-python.sh");
            println!("cargo:rerun-if-changed={}", script_path.to_str().unwrap());
            let command = if cfg!(target_os = "windows") { env::var("MSYS2_BASH").unwrap_or_else(|_| "bash".to_string()) } else { "bash".to_string() };
            let status = Command::new(&command).arg(&script_path).status().expect("Failed to execute bundle-python.sh script");
            if !status.success() {
                panic!("Python bundling script failed with exit code: {}", status);
            }
        } else {
            println!("cargo:info=Skipping Python bundling: python directory already exists for dev build.");
        }
    }

    // --- Copy Python scripts to target/debug/scripts for development (existing logic) ---
    let scripts_source_dir = manifest_dir.join("scripts");
    let scripts_target_dir = manifest_dir.join("target").join(env::var("PROFILE").unwrap()).join("scripts");
    fs::create_dir_all(&scripts_target_dir).expect("Failed to create target scripts directory");

    let python_scripts_to_copy = vec!["convert_with_pandoc.py"];

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

    println!("cargo:rerun-if-changed=build.rs");
    tauri_build::build();
    Ok(())
}

fn get_target_platform_string() -> Result<String> {
    let target_triple = env::var("TARGET").context("TARGET environment variable not set")?;
    let platform = match target_triple.as_str() {
        "x86_64-apple-darwin" => "macos-x86_64",
        "aarch64-apple-darwin" => "macos-arm64",
        "x86_64-pc-windows-msvc" | "x86_64-pc-windows-gnu" => "windows-x86_64",
        "aarch64-pc-windows-msvc" => "windows-arm64",
        "x86_64-unknown-linux-gnu" => "linux-x86_64",
        "aarch64-unknown-linux-gnu" => "linux-arm64",
        _ => anyhow::bail!("Unsupported target triple: {}", target_triple),
    };
    Ok(platform.to_string())
}

fn rename_whisper_binaries_for_tauri(sidecars_dir: &Path) -> Result<()> {
    let target_triple = env::var("TARGET").context("TARGET environment variable not set")?;
    let exe_suffix = if target_triple.contains("windows") { ".exe" } else { "" };

    let binaries_to_rename = vec!["whisper-cli", "whisper-stream"];

    for bin_name in binaries_to_rename {
        let old_name = format!("{}{}", bin_name, exe_suffix);
        let new_name = format!("{}-{}{}", bin_name, target_triple, exe_suffix);

        let old_path = sidecars_dir.join(&old_name);
        let new_path = sidecars_dir.join(&new_name);

        if old_path.exists() {
            fs::rename(&old_path, &new_path).with_context(|| {
                format!("Failed to rename binary from {} to {}", old_path.display(), new_path.display())
            })?;
            println!("cargo:info=Renamed {} to {}", old_name, new_name);
        } else {
            // This could happen if the file was already renamed, so we just log a warning.
            println!("cargo:warning=Binary to rename not found at {}. It might have been already renamed.", old_path.display());
        }
    }

    Ok(())
}


fn download_and_unzip(asset_name: &str, platform: &str, dest_dir: &Path) -> Result<()> {
    let url = format!("{}/{}/{}-{}.zip", SIDECARS_BASE_URL, SIDECAR_VERSION, asset_name, platform);
    println!("cargo:info=Downloading {} from {}", asset_name, url);

    let agent = ureq::builder().build();
    let response = agent.get(&url).call().with_context(|| format!("Failed to download from {}", url))?;

    if response.status() != 200 {
        anyhow::bail!("Failed to download from {}: HTTP {}", url, response.status());
    }

    let mut bytes = Vec::new();
    response.into_reader().read_to_end(&mut bytes)?;

    let cursor = io::Cursor::new(bytes);
    let mut archive = ZipArchive::new(cursor)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let outpath = dest_dir.join(file.mangled_name());

        if file.name().ends_with('/') {
            fs::create_dir_all(&outpath)?;
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    fs::create_dir_all(&p)?;
                }
            }
            let mut outfile = File::create(&outpath)?;
            io::copy(&mut file, &mut outfile)?;
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Some(mode) = file.unix_mode() {
                if mode & 0o111 != 0 {
                     fs::set_permissions(&outpath, fs::Permissions::from_mode(mode))?;
                }
            }
        }
    }
    Ok(())
}
