// src-tauri/build.rs

use anyhow::{Context, Result};
use std::env;
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use zip::ZipArchive;
use bzip2::read::BzDecoder;
use tar::Archive;



#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

const SIDECAR_VERSION: &str = "v0.2.0";
const SIDECARS_BASE_URL: &str = "https://github.com/dipanjan92/harvey-sidecars/releases/download";

fn main() -> Result<()> {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let python_dir = manifest_dir.join("python");
    let binaries_dir = manifest_dir.join("binaries");
    fs::create_dir_all(&python_dir).expect("failed to create python dir");
    fs::create_dir_all(&binaries_dir).expect("failed to create binaries dir");
    let sidecars_dir = manifest_dir.join("sidecars");
    let profile = env::var("PROFILE").unwrap();
    let target_triple = env::var("TARGET").context("TARGET environment variable not set")?;

    let exe_suffix = if target_triple.contains("windows") { ".exe" } else { "" };
    let whisper_cli_path = sidecars_dir.join(format!("whisper-cli{}", exe_suffix));

    if profile == "release" || !whisper_cli_path.exists() {
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
            "cargo:warning=Detected target platform: {}",
            target_platform
        );

        if sidecars_dir.exists() {
            fs::remove_dir_all(&sidecars_dir)?;
        }
        fs::create_dir_all(&sidecars_dir)?;

        if target_triple.contains("windows") {
            download_whisper_for_windows(&sidecars_dir)?;
        } else {
            let whisper_asset_name = format!("whisper-sidecars-{}", target_platform);
            download_and_unzip(&whisper_asset_name, &sidecars_dir)?;
        }

        rename_sidecar_binaries_for_tauri(&sidecars_dir)?;

        println!("cargo:info=Sidecars downloaded and extracted successfully.");
    } else {
        println!("cargo:info=Skipping sidecar download: sidecars directory already exists for dev build.");
    }

    bundle_micromamba()?;

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
        // For Windows ARM64, we download the x64 binary and run it via emulation.
        "aarch64-pc-windows-msvc" => "win-64", 
        _ => anyhow::bail!("Unsupported target triple for Micromamba: {}", target_triple),
    };

    let generic_binary_name = "micromamba";
    let exe_suffix = if target_triple.contains("windows") { ".exe" } else { "" };

    // The binary is named after the target triple, which is what Tauri expects.
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
    
    println!("cargo:info=Micromamba for {} downloaded successfully to {}", target_triple, platform_path.display());

    Ok(())
}

fn get_target_platform_string() -> Result<String> {
    let target_triple = env::var("TARGET").context("TARGET environment variable not set")?;
    let platform = match target_triple.as_str() {
        "x86_64-apple-darwin" => "macos-x86_64",
        "aarch64-apple-darwin" => "macos-arm64",
        "x86_64-pc-windows-msvc" | "x86_64-pc-windows-gnu" => "windows-x86_64",
        // For Windows ARM64, we use the x64 sidecars and run them via emulation.
        "aarch64-pc-windows-msvc" => "windows-arm64",
        "x86_64-unknown-linux-gnu" => "linux-x86_64",
        "aarch64-unknown-linux-gnu" => "linux-arm64",
        _ => anyhow::bail!("Unsupported target triple: {}", target_triple),
    };
    Ok(platform.to_string())
}

fn rename_sidecar_binaries_for_tauri(sidecars_dir: &Path) -> Result<()> {
    let target_triple = env::var("TARGET").context("TARGET environment variable not set")?;
    let exe_suffix = if target_triple.contains("windows") {
        ".exe"
    } else {
        ""
    };

    let binaries_to_rename = vec!["whisper-cli", "whisper-stream"];

    // if target_triple == "aarch64-pc-windows-msvc" {
    //     binaries_to_rename.push("ffmpeg");
    // }

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


fn download_and_unzip(asset_name: &str, dest_dir: &Path) -> Result<()> {


    let url = format!(


        "{}/{}/{}.zip",


        SIDECARS_BASE_URL, SIDECAR_VERSION, asset_name


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


            let file = archive.by_index(i)?;


            println!("cargo:info=Found file in zip: {}", file.name());


        }


    


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





    // --- Post-extraction adjustments for FFmpeg on Windows ---


    // if asset_name == "ffmpeg-win-arm64" || asset_name == "ffmpeg-windows-x86_64" {


    //     let bin_dir = dest_dir.join("bin");


    //     if bin_dir.exists() && bin_dir.is_dir() {


    //         for entry in fs::read_dir(bin_dir)? {


    //             let entry = entry?;


    //             let src_path = entry.path();


    //             let dest_path = dest_dir.join(src_path.file_name().unwrap());


    //             fs::rename(&src_path, &dest_path)?;


    //         }


    //         fs::remove_dir_all(dest_dir.join("bin"))?;


    //     }


    // }





    Ok(())


}

    

        


fn download_whisper_for_windows(dest_dir: &Path) -> Result<()> {
    let asset_name = "whisper-blas-bin-x64.zip";
    let url = format!(
        "https://github.com/ggml-org/whisper.cpp/releases/latest/download/{}",
        asset_name
    );

    println!("cargo:info=Downloading {} for Windows from {}", asset_name, url);

    let agent = ureq::builder().build();
    let response = agent.get(&url).call().with_context(|| format!("Failed to download from {}", url))?;

    if response.status() != 200 {
        anyhow::bail!("Failed to download from {}: HTTP {}", url, response.status());
    }

    let mut bytes = Vec::new();
    response.into_reader().read_to_end(&mut bytes)?;

    let cursor = io::Cursor::new(bytes);
    let mut archive = ZipArchive::new(cursor)?;

    let temp_extract_dir = dest_dir.join("_temp_extract");
    if temp_extract_dir.exists() {
        fs::remove_dir_all(&temp_extract_dir)?;
    }
    fs::create_dir_all(&temp_extract_dir)?;

    archive.extract(&temp_extract_dir)?;

    // The files are inside a "Release" subdirectory in the archive.
    let files_to_copy = vec![
        "whisper-cli.exe",
        "whisper-stream.exe",
        "SDL2.dll",
        "libopenblas.dll",
        "ggml-base.dll",
        "ggml-blas.dll",
        "ggml-cpu.dll",
        "ggml.dll",
        "whisper.dll",
    ];

    let release_dir = temp_extract_dir.join("Release");

    if !release_dir.exists() {
        anyhow::bail!("'Release' directory not found in the downloaded archive.");
    }

    for file_name in files_to_copy {
        let src_path = release_dir.join(file_name);
        let dest_path = dest_dir.join(file_name);

        if src_path.exists() {
            fs::rename(&src_path, &dest_path).with_context(|| {
                format!("Failed to move {} to {}", src_path.display(), dest_path.display())
            })?;
            println!("cargo:info=Copied {} to {}", src_path.display(), dest_path.display());
        } else {
            anyhow::bail!("Expected file {} not found in the archive's 'Release' directory.", src_path.display());
        }
    }

    fs::remove_dir_all(&temp_extract_dir)?;

    Ok(())
}