// src-tauri/build.rs

use anyhow::{Context, Result};
use std::env;
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use zip::ZipArchive;
use bzip2::read::BzDecoder;
use tar::Archive;
use serde_json::Value;
use flate2::read::GzDecoder;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

const SIDECAR_VERSION: &str = "v0.2.0";
const SIDECARS_BASE_URL: &str = "https://github.com/dipanjan92/harvey-sidecars/releases/download";

fn main() -> Result<()> {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let sidecars_dir = manifest_dir.join("sidecars");
    let profile = env::var("PROFILE").unwrap();
    let target_triple = env::var("TARGET").context("TARGET environment variable not set")?;

    if profile == "release" || !sidecars_dir.exists() {
        let reason = if profile == "release" {
            "release build"
        } else {
            "dev build and sidecars are missing"
        };
        println!(
            "cargo:info=Downloading pre-compiled sidecars (reason: {})...",
            reason
        );

        let target_platform = get_target_platform_string()?;
        println!(
            "cargo:info=Detected target platform: {}",
            target_platform
        );

        if sidecars_dir.exists() {
            fs::remove_dir_all(&sidecars_dir)?;
        }
        fs::create_dir_all(&sidecars_dir)?;

        download_and_unzip("whisper-sidecars", &target_platform, &sidecars_dir)?;

        rename_whisper_binaries_for_tauri(&sidecars_dir)?;

        println!("cargo:info=Sidecars downloaded and extracted successfully.");
    } else {
        println!("cargo:info=Skipping sidecar download: sidecars directory already exists for dev build.");
    }

    if target_triple == "aarch64-pc-windows-msvc" {
        bundle_python_standalone()?;
    } else {
        bundle_micromamba()?;
    }

    // --- Copy Python scripts to target/debug/scripts for development (existing logic) ---
    let scripts_source_dir = manifest_dir.join("scripts");
    let scripts_target_dir = manifest_dir
        .join("target")
        .join(env::var("PROFILE").unwrap())
        .join("scripts");
    fs::create_dir_all(&scripts_target_dir).expect("Failed to create target scripts directory");

    let python_scripts_to_copy = vec!["convert_with_pandoc.py"];

    for script_name in python_scripts_to_copy {
        let source_path = scripts_source_dir.join(script_name);
        let dest_path = scripts_target_dir.join(script_name);
        if source_path.exists() {
            fs::copy(&source_path, &dest_path).expect(&format!(
                "Failed to copy {} to {}",
                source_path.display(),
                dest_path.display()
            ));
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
        "aarch64-pc-windows-msvc" => {
            println!("cargo:warning=Micromamba does not support windows-arm64 yet. Skipping download.");
            return Ok(());
        }
        _ => anyhow::bail!("Unsupported target triple for Micromamba: {}", target_triple),
    };

    let generic_binary_name = "micromamba";
    let exe_suffix = if target_triple.contains("windows") { ".exe" } else { "" };
    let platform_path = binaries_dir.join(format!("{}-{}{}", generic_binary_name, target_triple, exe_suffix));

    if platform_path.exists() {
        println!("cargo:info=Micromamba for {} already exists. Skipping download.", target_triple);
        return Ok(());
    }

    let url = format!("https://micro.mamba.pm/api/micromamba/{}/latest", mamba_platform);
    println!("cargo:info=Downloading Micromamba for {} from {}", target_triple, url);

    let agent = ureq::builder().build();
    let response = agent.get(&url).call().with_context(|| format!("Failed to download from {}", url))?;

    if response.status() != 200 {
        anyhow::bail!("Failed to download from {}: HTTP {}", url, response.status());
    }

    let mut compressed_bytes = Vec::new();
    response.into_reader().read_to_end(&mut compressed_bytes)?;

    let bz_decoder = BzDecoder::new(compressed_bytes.as_slice());
    let mut archive = Archive::new(bz_decoder);

    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?;
        if let Some(file_name) = path.file_name() {
            if file_name.to_string_lossy() == "micromamba" {
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
    
    println!("cargo:info=Micromamba for {} downloaded successfully to {}", target_triple, platform_path.display());

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
    let exe_suffix = if target_triple.contains("windows") {
        ".exe"
    } else {
        ""
    };

    let binaries_to_rename = vec!["whisper-cli", "whisper-stream"];

    for bin_name in binaries_to_rename {
        let old_name = format!("{}{}", bin_name, exe_suffix);
        let new_name = format!("{}-{}{}", bin_name, target_triple, exe_suffix);

        let old_path = sidecars_dir.join(&old_name);
        let new_path = sidecars_dir.join(&new_name);

        if old_path.exists() {
            fs::rename(&old_path, &new_path).with_context(|| {
                format!(
                    "Failed to rename binary from {} to {}",
                    old_path.display(),
                    new_path.display()
                )
            })?;
            println!("cargo:info=Renamed {} to {}", old_name, new_name);
        } else {
            // This could happen if the file was already renamed, so we just log a warning.
            println!(
                "cargo:warning=Binary to rename not found at {}. It might have been already renamed.",
                old_path.display()
            );
        }
    }

    Ok(())
}

fn bundle_python_standalone() -> Result<()> {
    const PYTHON_VERSION: &str = "3.12.12";
    const PYTHON_BUILD_TAG: &str = "20251010";

    println!("cargo:info=Bundling standalone Python for Windows ARM64...");

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let python_dir = manifest_dir.join("python");
    let tmp_tar_path = manifest_dir.join("python-bundle.tar.gz");

    // --- Clean up previous builds ---
    if python_dir.exists() {
        fs::remove_dir_all(&python_dir)?;
    }
    if tmp_tar_path.exists() {
        fs::remove_file(&tmp_tar_path)?;
    }

    // --- Find Download URL via GitHub API ---
    let api_url = format!(
        "https://api.github.com/repos/astral-sh/python-build-standalone/releases/tags/{}",
        PYTHON_BUILD_TAG
    );

    println!("cargo:info=Querying GitHub API: {}", api_url);

    let agent = ureq::builder().build();
    let response = agent.get(&api_url).call()?;
    let json: Value = serde_json::from_reader(response.into_reader())?;

    let assets = json["assets"].as_array().context("No assets found in release")?;
    let target_platform_str = "aarch64-pc-windows-msvc";

    let download_url = assets
        .iter()
        .find_map(|asset| {
            let name = asset["name"].as_str()?;
            if name.contains(PYTHON_VERSION) && name.contains(target_platform_str) && name.ends_with("install_only.tar.gz") {
                asset["browser_download_url"].as_str().map(String::from)
            } else {
                None
            }
        })
        .context("Could not find a matching Python bundle URL from the GitHub API.")?;

    println!("cargo:info=Found download URL: {}", download_url);

    // --- Download and Extract Python ---
    println!("cargo:info=Downloading Python bundle...");
    let download_response = agent.get(&download_url).call()?;
    let mut bytes = Vec::new();
    download_response.into_reader().read_to_end(&mut bytes)?;
    fs::write(&tmp_tar_path, &bytes)?;

    println!("cargo:info=Extracting Python bundle...");
    let tar_gz = File::open(&tmp_tar_path)?;
    let tar = GzDecoder::new(tar_gz);
    let mut archive = Archive::new(tar);
    archive.unpack(&manifest_dir)?; // Unpacks into a 'python' directory

    // --- Clean up downloaded tarball ---
    fs::remove_file(&tmp_tar_path)?;

    println!("cargo:info=Self-contained Python has been bundled into {}", python_dir.display());

    Ok(())
}


fn download_and_unzip(asset_name: &str, platform: &str, dest_dir: &Path) -> Result<()> {
    let url = format!(
        "{}/{}/{}-{}.zip",
        SIDECARS_BASE_URL, SIDECAR_VERSION, asset_name, platform
    );
    println!("cargo:info=Downloading {} from {}", asset_name, url);

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
            if let Some(mode) = file.unix_mode() {
                if mode & 0o111 != 0 {
                    fs::set_permissions(&outpath, fs::Permissions::from_mode(mode))?;
                }
            }
        }
    }
    Ok(())
}