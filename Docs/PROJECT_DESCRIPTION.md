# Project Harvey 1.0: Architectural Overview

## Harvey 1.0: The Vision
Project Harvey 1.0 is a comprehensive desktop application designed for researchers, journalists, and individuals who work intensively with qualitative multimedia data. It provides a robust suite of offline tools for managing projects, transcribing audio and video files using local AI models (Whisper.cpp / Faster-Whisper), diarizing speakers (Pyannote), translating text (NLLB / Helsinki-NLP), and editing transcripts with a feature-rich, media-synchronized text editor.

Harvey 1.0 is designed with privacy and data sovereignty as a priority. Core AI functionalities run entirely locally on the user's hardware. Users download the necessary models once during the initial Setup Wizard and can then operate entirely offline, ensuring sensitive research data never leaves their device.

## Technology Stack

### Core Framework
*   **[Tauri](https://tauri.app/) (v2.0)**: The application framework providing the secure, lightweight webview container and native OS API bindings.
*   **Backend**: [Rust](https://www.rust-lang.org/) for high-performance file I/O, database management, and asynchronous shell command execution (managing Python subprocesses).
*   **Frontend**: [SvelteKit](https://kit.svelte.dev/) (using Svelte 4) for a highly reactive, component-driven user interface.

### Data Storage
*   **SQLite (`harvey.sqlite`)**: Managed via `rusqlite` in Rust, this centralized database acts as the source of truth for the project hierarchy, relational links (file groups, tags), and extensive metadata (including custom JSON fields and base64 thumbnails).
*   **`.harvey` Manifest**: A serialized JSON file utilized for project portability and backup state recovery.
*   **Global Configuration**: A `config.json` stored in the user's OS application data directory, managing paths to downloaded models and global preferences (like theme selection).

### Key Libraries
*   **Text Editor**: [Lexical](https://lexical.dev/) provides the extensible framework for the rich-text continuous document viewer and the segmented, timestamp-aware transcript editor.
*   **Table Display**: [Tabulator](https://tabulator.info/) for high-performance rendering of CSV/XLSX grids, supporting custom editors, history tracking, and nested sub-documents (Surveys).
*   **Image/PDF Viewing**: [OpenSeadragon](https://openseadragon.github.io/) (for deep-zoom image viewing), combined with custom Svelte SVG overlays for persistent, scalable annotations and censorship pixelation. [PDF.js](https://mozilla.github.io/pdf.js/) handles document rendering.
*   **Styling**: Tailwind CSS combined with Flowbite-Svelte components for a consistent, utility-first design system supporting native dark/light modes.

## Data Flow Architecture

The application strictly adheres to a unidirectional data flow pattern mediated by Tauri's Inter-Process Communication (IPC):

1.  **Frontend Interaction**: A user action in a Svelte component (e.g., clicking a file in `DataLeftPanel.svelte`) triggers a local event.
2.  **Service Layer Abstraction**: The component calls a centralized service (e.g., `loadRequestedItem` or `projectService.js`), which wraps the complex logic.
3.  **Tauri IPC `invoke`**: The service serializes the request and calls a registered Rust command via `@tauri-apps/api/core` (e.g., `invoke('get_asset_metadata_command', { assetRelativePath: path })`).
4.  **Rust Backend Execution**: The Tauri command routing in `src-tauri/src/main.rs` passes the payload to a specific module handler (e.g., `projectview/metadata_commands.rs`). The Rust handler queries the SQLite database or performs file system I/O.
5.  **State Update**: The Rust handler serializes the result (or an error string) and returns it to the Svelte service. The service parses the data and explicitly updates a specific global Svelte store (e.g., `$projectStore` or `$transcriptStore`).
6.  **Reactivity**: Bound Svelte components instantly re-render to reflect the new state, mounting new views or updating internal states without direct DOM manipulation.

## Component Documentation

Rather than maintaining an exhaustive list of every file in this document, Project Harvey enforces a **"Visual First" Documentation Standard** using colocated `README.md` files.

Every major directory (UI components, backend modules, stores, services, and routes) contains a `README.md` featuring Mermaid diagrams detailing visual wireframes, component architectures, IPC interop, and state dependencies.

To understand a specific feature, navigate directly to its source folder:
*   Frontend Components: `src/lib/components/`
*   Backend Modules: `src-tauri/src/`
*   Global State: `src/lib/stores/`
*   Routing Logic: `src/routes/`