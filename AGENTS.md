# Harvey Build Environment Setup

This document outlines the necessary steps to set up the build environment for the Harvey application on a Debian-based Linux system.

**Important Note:** These commands must be executed inside the specific build environment (e.g., within the Docker container, VM, or CI/CD runner) where the application build will take place.

> [!TIP]
> You can also use the automated bootstrapper to check your environment and install missing dependencies:
> ```bash
> bash scripts/bootstrap.sh
> ```

## 1. System Dependencies

First, ensure the package list is up to date within your build environment:

```bash
sudo apt-get update
```

Next, install the required libraries and build tools for Tauri. The command below installs the core system dependencies:

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
- `pypandoc` for document conversion (or `pypandoc_binary` on some platforms).

No manual Python setup is required by the user for most platforms, as the application handles the creation and management of this environment using a bundled version of Micromamba.

## 3. Development Setup

Before running or building the application, install the frontend dependencies:

```bash
npm install
```

## 4. Pre-Commit Checks

Before submitting any changes, ensure the following checks pass:

1.  **Svelte Check**:
    ```bash
    npm run check
    ```
2.  **Cargo Check**:
    ```bash
    cd src-tauri && cargo check
    ```

## 5. Building the Application

After installing the dependencies inside the build environment, you can build the Tauri application:

```bash
npm run tauri build
```

## 6. Troubleshooting

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

---

## 7. E2E Testing

Full end-to-end test docs live in `Docs/E2E_TESTING.md`. The critical rule to be aware of:

> [!CAUTION]
> **Never run the e2e test suite against a stale binary.**
> The tests launch the compiled native binary directly (`src-tauri/target/debug/harvey`), which has the entire `build/` directory **embedded at compile time**. If you run tests without first rebuilding both the frontend and the binary, the application logic and behavior reflected in the logs may be incorrect or outdated.

**Always run this command before executing `npx wdio run wdio.conf.mjs`:**
```bash
npm run tauri build -- --debug
```

---

## 8. Documentation is the Source of Truth
The Harvey application is complex, utilizing SvelteKit, Tauri IPC, SQLite, and Python subprocesses. It relies on a strictly enforced **"Visual First" documentation standard**.

Before attempting to implement a new feature, modify a component, or debug an issue, you **MUST**:
1. Identify the directory housing the target code (e.g., `src/lib/components/projectview/data/`).
2. Read the colocated `README.md` file within that directory to understand the component architecture, required Props, Svelte Stores, and specific Tauri IPC Commands.
3. Review `Docs/DATABASE_SCHEMA.md` if your task involves modifying database schemas or Rust `db_handler.rs` queries.

Do not attempt to infer complex component lifecycles or store dependencies solely by reading the source code without first consulting the relevant `README.md`.

## 8. Mandatory Documentation Updates
When you modify code that alters the component architecture, adds new Props, introduces new Svelte Stores, invokes a new Tauri backend command, or changes a database table, you **MUST**:
1. Locate the corresponding `README.md` file (or `Docs/DATABASE_SCHEMA.md`).
2. Update the Mermaid diagrams (e.g., `block-beta`, `flowchart TD`, `erDiagram`) to reflect your changes.
3. Update the textual breakdown (Props, State, IPC Commands).
4. Ensure your updates strictly conform to the templates defined in `CONTRIBUTING.md`.

You are not finished with a feature or bug fix until the documentation is updated to match the new code state.

## 9. Styling Constraints
* Use **Tailwind CSS** utility classes directly in the markup.
* Use **Flowbite-Svelte** components for interactive UI elements (Modals, Inputs, Buttons) where possible.
* Support Dark Mode by explicitly defining `dark:` variant classes alongside light mode classes (e.g., `bg-white dark:bg-gray-900 text-gray-900 dark:text-gray-100`).
* Refer to `Docs/STYLE_GUIDE.md` for the core color palette mapping.