# Harvey Build Environment Setup

This document outlines the necessary steps to set up the build environment for the Harvey application on a Debian-based Linux system.

**Important Note:** These commands must be executed inside the specific build environment (e.g., within the Docker container, VM, or CI/CD runner) where the application build will take place.

## 1. System Dependencies

First, ensure the package list is up to date within your build environment:

```bash
sudo apt-get update
```

Next, install the required libraries and build tools. This single command installs everything needed for the Tauri backend on Linux:

```bash
sudo apt-get install -y \
    build-essential \
    curl \
    wget \
    file \
    libssl-dev \
    libglib2.0-dev \
    libgtk-3-dev \
    libayatana-appindicator3-dev \
    librsvg2-dev \
    libsoup-3.0-dev \
    libwebkit2gtk-4.1-dev
```

## 2. Python Virtual Environment

The application backend creates and manages a self-contained Python virtual environment (`harvey_env`) within the app's configuration directory. This environment is used for running tasks that rely on Python libraries, such as audio processing and document conversion.

Core dependencies are installed automatically by the application on first launch or when required. Key libraries include:
- `torch` and `torchaudio` for audio processing.
- `pyannote.audio` for speaker diarization.
- `pypandoc_binary` for document conversion.

No manual Python setup is required by the user, as the application handles the creation and management of this environment.

## 3. Building the Application

After installing the dependencies inside the build environment, you can build the Tauri application:

```bash
npm run tauri build
```

## 3. Troubleshooting

If the build fails with an error like "The system library ... was not found," it almost always means the dependency was not installed correctly inside the build environment.

### Step A: Verify the Installation

Before trying anything else, verify that the required library is actually visible to the `pkg-config` tool from your build shell:

```bash
# This command should output a version number (e.g., 2.44.2).
# If it says the package was not found, the installation in step 1 failed or was done in the wrong environment.
pkg-config --modversion webkit2gtk-4.1
```

### Step B: Check `PKG_CONFIG_PATH`

On standard Debian systems, setting `PKG_CONFIG_PATH` is not usually necessary if the `-dev` packages were installed correctly. However, if the verification in Step A passed but the build still fails, you can explicitly set the path as a last resort:

```bash
export PKG_CONFIG_PATH=/usr/lib/x86_64-linux-gnu/pkgconfig
npm run tauri build
```