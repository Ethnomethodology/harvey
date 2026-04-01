# Project Harvey - Setup Guide

Welcome to the Project Harvey setup guide! This document covers the essential steps required to get the development version of the application running on your local machine, and subsequently, how to configure the internal AI models and Python dependencies via the in-app Setup Wizard.

## Part 0: Quick Start (Automated Setup)

On **Linux** and **macOS**, you can use our automated bootstrapper to check your environment, install most system dependencies, and build the application in one step:

```bash
curl -sSL https://raw.githubusercontent.com/Ethnomethodology/harvey/main/scripts/bootstrap.sh | bash
```

Alternatively, if you have already cloned the repository:

```bash
bash scripts/bootstrap.sh
```

The script will guide you through any missing requirements (like Rust or Node.js) and handle the compilation process for you.

## Part 1: Installing System Prerequisites

Harvey relies on a robust dual-stack architecture: a SvelteKit (Node.js) frontend and a Rust (Tauri) backend. You must install the prerequisites for both environments.

### 1.1. Node.js & npm (Frontend)
1. Download and install the latest **LTS (Long-Term Support)** version of Node.js from [nodejs.org](https://nodejs.org/).
2. This installation includes `npm` (Node Package Manager), which is required to manage the frontend JavaScript dependencies and the local Tauri CLI.
3. Verify your installation by running `node -v` and `npm -v` in your terminal.

### 1.2. Rust & Cargo (Backend)
1. Install Rust and its package manager, Cargo, using `rustup` by visiting [rustup.rs](https://rustup.rs/) and following the instructions for your operating system.
2. Verify your installation by running `rustc --version` and `cargo --version`.

### 1.3. Tauri OS-Specific Build Tools (Crucial)
Tauri v2 requires specific native libraries to compile the application and launch the system webview. **If you skip this step, the project will fail to build.**

Follow the official Tauri prerequisites guide for your specific operating system carefully:
*   [Tauri v2 Prerequisites Guide](https://v2.tauri.app/develop/guides/prerequisites/)
*   **Windows**: Requires the "Desktop development with C++" workload via the Visual Studio Build Tools.
*   **macOS**: Requires the Xcode Command Line Tools (`xcode-select --install`).
*   **Linux**: Requires several `gtk` and `webkit2gtk` development packages depending on your distribution (e.g., Ubuntu/Debian).

---

## Part 2: Building and Running the Project

Once the system prerequisites are installed, you can pull down the source code and launch the development server.

1. **Clone the Repository**:
   Download the source code using Git (or GitHub Desktop):
   ```bash
   git clone https://github.com/Ethnomethodology/harvey.git
   cd harvey
   ```

2. **Install Frontend Dependencies**:
   This command installs all SvelteKit packages, Tailwind CSS, Flowbite, and the project-local Tauri CLI (`@tauri-apps/cli`).
   ```bash
   npm install
   ```

3. **Launch Development Mode**:
   Start the application in development mode with hot-module reloading enabled. The first run will take several minutes as Cargo downloads and compiles all Rust crates.
   ```bash
   npm run tauri dev
   ```

*(Optional)* To create a standalone production build (e.g., an `.exe` or `.app`), run `npm run tauri build`. The output will be located in `src-tauri/target/release/bundle/`.

---

## Part 3: The In-App Setup Wizard (AI & Python Configuration)

Project Harvey relies on local machine learning models (Whisper, Pyannote, NLLB, Helsinki) for transcription, diarization, and translation. These models require Python and specific libraries to function.

When you launch Harvey for the first time, you will see the **Welcome Screen**. If your system is not yet configured, a yellow **Configuration Required** banner will appear.

### Running the Setup Wizard
1. Click the **Launch Setup Wizard** button on the Welcome Screen. This wizard automates the complex installation of Python environments and ML models.
2. **Step 1: Python Libraries**
   * Harvey uses a bundled version of **Micromamba** to create and manage a completely isolated, self-contained Python environment (`harvey_env`) within its configuration directory.
   * **No manual Python installation is required** on your system. Harvey will automatically install **Python 3.12** and all necessary libraries like `faster-whisper`, `torch`, `pyannote.audio`, and `transformers` within this dedicated environment.
   * You can monitor the live terminal output in the installation log modal.
3. **Step 2: Hugging Face Token (Diarization)**
   * To identify different speakers (diarization), Harvey uses `pyannote`. This requires a free Hugging Face account and a User Access Token.
   * Enter your token. The wizard will securely validate it against the Hugging Face API and save it to your local configuration.
4. **Step 3: Download Models**
   * The wizard will prompt you to download the core AI models required for transcription and translation to your local machine.
   * You can customize the download location in the configuration settings later.

### Manual Configuration
If you prefer, you can skip the wizard and configure these settings manually by switching to the **Configure** tab on the Welcome Screen. Here you can change your download paths, manage installed libraries, input your Hugging Face token, and toggle specific transcription engines (e.g., switching between Whisper.cpp and Faster-Whisper).