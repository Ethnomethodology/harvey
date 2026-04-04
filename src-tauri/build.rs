// src-tauri/build.rs

use anyhow::{Context, Result};
use bzip2::read::BzDecoder;
use std::env;
use std::fs;
use std::io::Read;
use std::path::PathBuf;
use tar::Archive;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

fn main() -> Result<()> {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let python_dir = manifest_dir.join("python");
    let binaries_dir = manifest_dir.join("binaries");
    fs::create_dir_all(&python_dir).expect("failed to create python dir");
    fs::create_dir_all(&binaries_dir).expect("failed to create binaries dir");

    bundle_micromamba()?;

    // --- Copy Python scripts to target/debug/scripts for development (existing logic) ---
    let scripts_source_dir = manifest_dir.join("scripts");
    let profile = env::var("PROFILE").unwrap();
    let scripts_target_dir = manifest_dir.join("target").join(&profile).join("scripts");
    fs::create_dir_all(&scripts_target_dir).expect("Failed to create target scripts directory");

    let python_scripts_to_copy = vec!["convert_with_pandoc.py"];

    for script_name in python_scripts_to_copy {
        let source_path = scripts_source_dir.join(script_name);
        let dest_path = scripts_target_dir.join(script_name);
        if source_path.exists() {
            fs::copy(&source_path, &dest_path).unwrap_or_else(|_| {
                panic!(
                    "Failed to copy {} to {}",
                    source_path.display(),
                    dest_path.display()
                )
            });
            println!(
                "cargo:info=Copied {} to {}",
                source_path.display(),
                dest_path.display()
            );
        } else {
            println!(
                "cargo:warning=Python script {} not found at {}",
                script_name,
                source_path.display()
            );
        }
    }

    println!("cargo:rerun-if-changed=build.rs");
    tauri_build::build();
    Ok(())
}

fn bundle_micromamba() -> Result<()> {
    println!("cargo:info=Bundling Micromamba executable...");

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let binaries_dir = manifest_dir.join("binaries");
    fs::create_dir_all(&binaries_dir)?;

    let target_triple = env::var("TARGET").context("TARGET environment variable not set")?;

    let mamba_platform = match target_triple.as_str() {
        "x86_64-apple-darwin" => "osx-64",
        "aarch64-apple-darwin" => "osx-arm64",
        "x86_64-pc-windows-msvc" | "x86_64-pc-windows-gnu" => "win-64",
        "x86_64-unknown-linux-gnu" => "linux-64",
        // For Windows ARM64, we download the x64 binary and run it via emulation.
        "aarch64-pc-windows-msvc" => "win-64",
        _ => anyhow::bail!(
            "Unsupported target triple for Micromamba: {}",
            target_triple
        ),
    };

    let generic_binary_name = "micromamba";
    let exe_suffix = if target_triple.contains("windows") {
        ".exe"
    } else {
        ""
    };

    // The binary is named after the target triple, which is what Tauri expects.
    let platform_path = binaries_dir.join(format!(
        "{}-{}{}",
        generic_binary_name, target_triple, exe_suffix
    ));

    if platform_path.exists() {
        println!(
            "cargo:info=Micromamba for {} already exists. Skipping download.",
            target_triple
        );
        return Ok(());
    }

    let url = format!(
        "https://micro.mamba.pm/api/micromamba/{}/latest",
        mamba_platform
    );
    println!(
        "cargo:info=Downloading Micromamba for {} from {}",
        target_triple, url
    );

    let agent = ureq::builder().build();
    let response = agent
        .get(&url)
        .call()
        .with_context(|| format!("Failed to download from {}", url))?;

    if response.status() != 200 {
        anyhow::bail!(
            "Failed to download from {}: HTTP {}",
            url,
            response.status()
        );
    }

    let mut compressed_bytes = Vec::new();
    response.into_reader().read_to_end(&mut compressed_bytes)?;

    let bz_decoder = BzDecoder::new(compressed_bytes.as_slice());
    let mut archive = Archive::new(bz_decoder);

    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?;
        // The binary inside the tarball is named "micromamba.exe" on Windows
        if let Some(file_name) = path.file_name() {
            if file_name.to_string_lossy().starts_with("micromamba") {
                let mut decompressed_bytes = Vec::new();
                entry.read_to_end(&mut decompressed_bytes)?;
                fs::write(&platform_path, &decompressed_bytes)?;
                break;
            }
        }
    }

    #[cfg(unix)]
    {
        fs::set_permissions(&platform_path, fs::Permissions::from_mode(0o755))?;
    }

    println!(
        "cargo:info=Micromamba for {} downloaded successfully to {}",
        target_triple,
        platform_path.display()
    );

    Ok(())
}
