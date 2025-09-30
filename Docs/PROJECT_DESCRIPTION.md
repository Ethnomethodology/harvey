# Project Harvey 1.0: Module Breakdown

## Harvey 1.0: An Overview
Project Harvey 1.0 is a comprehensive desktop application designed for researchers, journalists, and individuals who work with multimedia content. The application is particularly aimed at academic and qualitative researchers. It provides a robust suite of tools for managing projects, transcribing audio and video files using local AI models, editing transcripts with a feature-rich text editor, and handling various related documents and media such as PDFs, text files, images, and tables. The application aims to streamline the workflow of analyzing qualitative data by integrating these functionalities into a cohesive, cross-platform experience (Windows, macOS, Linux) built with Tauri and Rust for performance and reliability.

Harvey 1.0 is designed with privacy as a priority; core AI functionalities like transcription and diarization run locally on the user's computer. Users download the necessary AI models once and can then use them repeatedly offline, ensuring their data remains on their device. It is an application built by researchers, for researchers.

## Technology Stack
*   **Core Framework**: [Tauri](https://tauri.app/) (v2.0, Rust backend, webview frontend)
*   **Backend Language**: [Rust](https://www.rust-lang.org/)
*   **Frontend Framework**: [SvelteKit](https://kit.svelte.dev/)
*   **Frontend Languages**: JavaScript (SvelteKit), HTML, CSS
*   **Text Editor**: [Lexical](https://lexical.dev/)
*   **PDF Viewing**: [PDF.js](https://mozilla.github.io/pdf.js/)
*   **Table Display**: [Tabulator](https://tabulator.info/)
*   **Image Viewing/Manipulation**: OpenSeadragon (viewing), Annotorious (annotations)
*   **Annotations Database**: SQLite (managed via `rusqlite` in Rust)
*   **UI Styling**: Tailwind CSS (based on `tailwind.config.js`)
*   **Tauri CLI**: Managed as a project dependency via npm (using `@tauri-apps/cli`)
*   **Sidecar Execution**: For local AI models (e.g., Whisper, Diarization tools)

## Key Features

*   **Comprehensive Project Management**: Create, open, import, and manage multimedia research projects using `.harvey` project files.
*   **Media Transcription**:
*   **Interactive Transcript Editor**: Edit and refine transcripts with a rich-text editor (Lexical-based) linked to media playback, including timestamp adjustment and speaker labeling.
*   **Versatile Document Handling**:
    *   **Rich Text Documents**: Create and edit data and documents with formatting, tables, and lists.
    *   **PDF Viewing & Annotation**: Open and view PDF documents, with robust text-based annotation capabilities (highlights, comments) that are stored and re-applied accurately. Annotations are saved in a separate SQLite database (`harvey_annotations.sqlite`).
    *   **Table Viewing**: Import and view CSV and XLSX files with interactive features like sorting and filtering.
    *   **Image Handling & Annotation**: Import, view (using OpenSeadragon), and annotate (using Annotorious) various image formats. Annotations can be saved and reloaded, managed by the backend `image_handler.rs`.
*   **Asset Management**: Organize various project assets including media files, transcripts, documents, images, and tables within a structured project environment.
*   **Configurable Settings**: Customize application behavior, including transcription model selection, download locations, and UI themes (light/dark/system) via `config.xml`.
*   **Cross-Platform**: Designed to run on Windows, macOS, and Linux.
*   **Data Export**: Export transcripts to common formats (e.g., DOCX).
*   **Background Processing**: Handles intensive tasks like model downloads and transcriptions in the background with progress tracking and cancellation support.

## Data Flow and Storage

**General Data Flow:**

User interactions in the SvelteKit frontend typically trigger functions within UI components. These components often utilize the central `projectService.js` (or other specific services like `configureActions.js`) to communicate with the backend. The service layer then uses Tauri's `invoke` mechanism to call the appropriate Rust command handlers defined in `src-tauri/lib.rs`. The Rust backend processes the request, interacts with the file system, project files, or the annotations database as needed, and returns a response to the frontend, which then updates the UI reactively.

**Primary Data Storage Methods:**

*   **Project Configuration & Structure**:
    *   **`project.harvey` (XML)**: Each project has a root XML file (e.g., `my_project.harvey`). This file defines the project's name, structure, and metadata about its associated files and assets (media, documents, tables, images, imported transcripts, etc.). It acts as the primary manifest for the project.
    *   *(Self-correction: The original README used `project.xml` in some places, and the term "project XML" was used generically. It's now clarified that `.harvey` is the specific project manifest file extension, containing XML data. This file is central to defining project structure and assets.)*
*   **Application-Wide Configuration**:
    *   **`config.xml` (User Configuration Directory)**: Stores global application settings, such as recent project paths, download locations for AI models, and theme preferences. This file is typically located in the user's application configuration directory (e.g., `~/.config/harvey_de_sitter/config.xml` on Linux, or platform-equivalent paths on Windows/macOS).
*   **Annotations Data**:
    *   **`harvey_annotations.sqlite` (User Configuration Directory)**: An SQLite database used to store detailed annotations for various document types (e.g., PDFs, images). This approach keeps annotation data separate from the main project file, allowing for efficient management of potentially complex annotation information. Annotations are linked to specific documents via their file paths and a `document_type` identifier.
*   **Project Assets**:
    *   **Project Folder**: All actual media files (audio, video), imported documents (original `.docx`, `.pdf`, `.txt`, and their `.json` representations for the editor), tables (`.csv`, `.xlsx`), images, and generated transcripts (`.json`) are stored within the project's dedicated folder. This folder is typically organized into subdirectories like `harvey_files/Media/`, `harvey_files/Documents/`, `harvey_files/Images/`, etc., to maintain a clear structure.
*   **Local AI Models**:
    *   **User-Defined Download Location**: Transcription models (e.g., for Whisper) are downloaded to a user-specified location on their local file system. This location is managed via the application's configuration settings stored in `config.xml`.

This document provides a detailed breakdown of the SvelteKit frontend components and their associated Rust backend modules for the Project Harvey 1.0 application.

---

## 1. Welcome Screen & Configuration (`src/lib/components/welcome/`)

* **Frontend Components**:
    * `WelcomeScreen.svelte`: Main entry point for project management and application configuration.
    * `ProjectList.svelte` & `ProjectItem.svelte`: Display the list of recent projects.
    * `Configure.svelte`: UI for application settings (download locations, models, theme).
    * `RenameModal.svelte`: Modal for renaming projects.
    * `actions.js`: Contains client-side logic for invoking backend commands related to this section.
* **Functionality**:
    * Manages the creation, opening, renaming, listing, importing, and deletion of transcription projects.
    * Handles global application configuration, including download paths for transcription models, theme preferences, and management of downloaded models.
* **Associated Backend (`src-tauri/src/welcome/`)**:
    * `commands.rs`:
        * Project actions: `load_recent_projects`, `create_project`, `rename_project`, `remove_project_from_list`, `open_project`, `import_project`, `delete_project`, `locate_in_finder`.
        * Configuration: `ensure_directory`, `save_download_location`, `get_download_location`, `get_downloaded_models`, `delete_model`, `download_model_command`, `cancel_download_command`, `change_download_location_and_move_models`,  `get_theme_preference`, `set_theme_preference`.
    * `config.rs`: Handles the reading and writing of the global `config.xml` file which stores recent projects and settings.

---

## 2. Project View (`src/routes/projectview/` & `src/lib/components/projectview/`)

This is the main multi-tab interface for working within a project.

#### Overall Structure & Navigation

* **Frontend Components**:
    * `ProjectView.svelte` (in `src/routes/projectview/`): The root component for the project workspace, likely managing the "Transcriptions" and "Data" tabs.
    * `BottomBar.svelte` (in `src/lib/components/projectview/shared/`): A shared component for displaying status messages or common actions across project views.
* **Functionality**:
    * Provides the main tabbed interface for "Transcriptions" (media-based) and "Data" (asset management).
    * Loads initial project data.
* **Associated Backend**:
    * `projectview/core_commands.rs`: `load_project_data` is crucial for populating the project view.
    * `projectview/shared_utils.rs`: For common utility functions used across project view modules.

---

#### Lexical Editor Core (`src/lib/components/projectview/lexical/`)

* **Frontend Components**:
    * `LexicalEditor.svelte`: The core Svelte component wrapping the Lexical rich-text editor.
    * `ColorPickerModal.svelte`: A modal for picking colors within the Lexical editor.
    * `ExtendedTextNode.js` (in `src/lib/nodes/`): Custom Lexical node to extend text functionalities, potentially for highlights or other annotations.
* **Functionality**:
    * Provides rich-text editing capabilities for documents and imported transcripts.
    * The custom `ExtendedTextNode` allows for features beyond standard text formatting.
* **Associated Backend**:
    * While the editor UI is frontend, the content it manages is persisted via backend commands:
        * Standard Documents (`.json`): `projectview/document_commands.rs` (e.g., `save_note_json`, `load_note_json`).
        * Imported Transcripts (edited as Lexical Table JSON): `projectview/transcription_handler.rs` (for initial `.docx` import via `import_word_transcript`), then `projectview/document_commands.rs` for saving/loading the resultant `.json` file (e.g., `save_note_json`, `load_note_json`).

---

#### Modals (`src/lib/components/projectview/modals/`)

* **Frontend Components**:
    * `ConfirmConversionModal.svelte`: For confirming document conversions (e.g., DOCX to Lexical JSON).
    * `ExportModal.svelte`: UI for exporting transcripts.
    * `FileRenameModal.svelte`: UI for renaming project items/assets.
    * `LinkModal.svelte`: For creating/editing hyperlinks within Lexical.
    * `ManageModelsModal.svelte`: UI for managing local transcription models.
    * `SpeakersModal.svelte`: UI for defining and assigning speaker names.
    * `TranscribeConfirmModal.svelte`: UI for confirming transcription settings before starting.
    * `UnsavedChangesModal.svelte`: Prompts user about unsaved changes.
* **Functionality**: Provides various dialogs for user interaction and configuration.
* **Associated Backend**:
    * `ExportModal.svelte`: `projectview/export_handler.rs` (specifically `export_transcript_to_docx`).
    * `FileRenameModal.svelte`: `projectview/core_commands.rs` (`rename_project_item`).
    * `ManageModelsModal.svelte`: `welcome/commands.rs` (uses model management commands like `get_downloaded_models`, `delete_model`, `download_model_command`).
    * `SpeakersModal.svelte`: `projectview/transcription_commands.rs` (`save_speaker_config`).
    * `TranscribeConfirmModal.svelte`:
        * Local Transcription: `projectview/local_handler/transcription.rs` (`run_transcription`, `cancel_transcription`).
        
    * Other modals primarily manage frontend state or simple confirmations.

---

#### Data View (`src/lib/components/projectview/notes/`)

This section handles the display and management of various project assets.

* **Frontend Components (Core Navigation & Structure)**:
    * `NotesLeftPanel.svelte`: Displays a tree-like list of project assets (Audios, Documents, Images, Tables, Imported Transcripts, Videos) and provides context menus (Open, Rename, Delete) and import buttons.
    * `NotesTopBar.svelte`: Toolbar for Fieldnotes actions like save and toggling autosave.
    * `NotesView.svelte`: Main container that conditionally renders the appropriate viewer/editor for the selected asset.
    * `NotesMiddlePanel.svelte`: (Present in file tree) Likely a layout component within `NotesView.svelte`, possibly for the main content area.
    * `shared_panels/LeftInfoPanel.svelte` & `shared_panels/RightInfoPanel.svelte`: Reusable side panels providing contextual information or tools for the currently viewed asset in Fieldnotes.
* **Functionality**:
    * Lists all project assets fetched from the project XML.
    * Allows users to open, rename, delete, and import various asset types.
    * Provides a central area to view and edit different types of Fieldnotes.
* **Associated Backend**:
    * `NotesLeftPanel.svelte`:
        * Loading asset lists: `projectview/core_commands.rs` (`load_project_data`).
        * Asset operations: `projectview/core_commands.rs` (`rename_project_item`, `delete_project_item`).
        * Import actions delegate to specific handlers (see below).
    * `NotesTopBar.svelte` (Save/Autosave actions):
        * Documents/Notes: `projectview/document_commands.rs` (`save_note_json`, `save_document_and_update_xml`).
        * Imported Transcripts: `projectview/document_commands.rs` (as they are saved as `.json` files, likely `save_note_json`).
        * Image Annotations: `projectview/image_handler.rs` (`save_image_annotations`).
        * PDF Annotations: `projectview/pdf_annotation_handler.rs` (`save_pdf_annotations`).
    * `shared_panels`: Backend calls would be contextual, e.g., fetching metadata via `projectview/document_commands.rs` (`load_document_metadata`) or asset-specific details.

---

* **Fieldnotes - Documents (`src/lib/components/projectview/notes/documents/`)**:
    * **Frontend Components**:
        * `DocumentView.svelte`: Main view for a selected document, hosts either the editor or PDF viewer.
        * `DocumentEditorPanel.svelte`: Lexical editor for `.json` documents (converted from `.txt`, `.md`, `.rtf`, non-transcript `.docx`).
        * `PDFViewerPanel.svelte`: Displays `.pdf` files using `pdf.js` and handles PDF annotations.
        * `PDFViewerPanel copy.svelte`: A duplicate or older version, likely not in active use if `PDFViewerPanel.svelte` is the primary.
        * `DocsLeftPanel.svelte` & `DocsRightPanel.svelte`: (Present in file tree) Likely older/refactored components, functionality presumably now in `notes/shared_panels/`. If active, they would show document-specific info.
    * **Functionality**:
        * Manages text-based documents (viewing, editing Lexical JSON) and PDFs (viewing, annotations).
        * Handles import and conversion of various document formats to Lexical JSON or direct copy for PDFs.
    * **Associated Backend**:
        * `DocumentEditorPanel.svelte`: `projectview/document_commands.rs` (`load_note_json` for loading, `save_note_json` or `save_document_and_update_xml` for saving).
        * `PDFViewerPanel.svelte`:
            * Initial PDF copy: `projectview/document_handler.rs` (`import_document`).
            * Annotation loading/saving: `projectview/pdf_annotation_handler.rs` (`load_pdf_annotations`, `save_pdf_annotations`).
            * Path/metadata display: `projectview/document_commands.rs`.
        * Document Import Process: `projectview/document_handler.rs` (`import_document` which uses Pandoc for conversions).
        * `DocsLeftPanel.svelte` / `DocsRightPanel.svelte` (if still used): `projectview/document_commands.rs`.

---

* **Fieldnotes - Tables (`src/lib/components/projectview/notes/tables/`)**:
    * **Frontend Components**:
        * `TableView.svelte`: Main view for a selected table.
        * `TableViewerPanel.svelte`: Displays `.csv` and `.xlsx` files using `Tabulator.js`.
    * **Functionality**: Imports and displays tabular data with features like sorting, filtering, and search.
    * **Associated Backend**:
        * `projectview/table_handler.rs`: `import_table_file` (for copying `.csv`/`.xlsx` files into the project) and `load_table_data` (for reading file content to display in Tabulator).

---

* **Fieldnotes - Images** (Frontend components `ImageView.svelte`, `ImageViewerPanel.svelte` described in summary):
    * **Functionality**: Imports and displays image files, with future support for annotations.
    * **Associated Backend**:
        * `projectview/image_handler.rs`: `import_image_file` (for copying image files), `load_image_annotations`, `save_image_annotations`.

---

* **Fieldnotes - Imported Transcripts** (Frontend components `ImportedTranscriptView.svelte`, `TranscriptEditorPanel.svelte` described in summary):
    * **Functionality**:
        * Imports externally generated transcripts (e.g., MS Word `.docx`).
        * Parses `.docx` to a simple JSON segment array initially.
        * Converts this simple JSON on-the-fly to Lexical Table JSON for display and editing in `TranscriptEditorPanel.svelte`.
        * Saves the full Lexical Table JSON back to the `.json` file.
    * **Associated Backend**:
        * `projectview/transcription_handler.rs`: `import_word_transcript` (for the initial `.docx` parsing to simple JSON).
        * `projectview/document_commands.rs`: `load_note_json` (to load the simple JSON or the full Lexical Table JSON from the `.json` file) and `save_note_json` (to save the full Lexical Table JSON back to the `.json` file).

---

#### Media-Based Transcriptions View (`src/lib/components/projectview/transcriptions/`)

This section handles transcripts generated *within* Harvey from audio/video media.

* **Frontend Components**:
    * `TranscriptionsView.svelte`: Main container for the media transcription interface.
    * `EditableTranscript.svelte`: Interactive transcript editor linked to media playback (likely Lexical-based).
    * `src/lib/components/projectview/shared/MediaPlayer.svelte`: A versatile component responsible for audio/video playback.
        *   When used within the "Transcriptions" view (without an `explicitMediaPath` prop), it interacts heavily with `transcriptStore.js` to play the currently selected media (`$transcriptStore.selectedMediaFile`), synchronize with its player state (`$transcriptStore.player`), and manage its audio buffer (`$transcriptStore.audioBuffer`). It uses functions from `transcriptStore.js` (like `updatePlayerTime`, `togglePlayerPlaying`) to update this shared state.
        *   It can also be instantiated with an `explicitMediaPath` prop (e.g., in the "Fieldnotes" view for media notes) to play specific media files independently of the main transcriptions view's state, managing its playback state locally in such cases.
        *   Provides UI controls for playback and includes logic for media loading, decoding, and error handling. It also supports functionalities like media trimming when used in the main transcriptions context.
    * `src/lib/components/projectview/shared/InteractiveWaveform.svelte`: Displays an interactive audio waveform.
    * `RichTextPreview.svelte`: Displays the transcript and includes a feature to "Convert Media Transcript to Document".
    * `LeftPanel.svelte`: A panel within this view, possibly for media files or settings.
    * `TopBar.svelte`: Toolbar specific to the transcriptions view.
    * `TreeNode.svelte`: Likely used for displaying hierarchical data if any (e.g. file trees if media selection is part of this view directly).
* **Functionality**:
    * Provides an interactive environment for editing transcripts generated from media files.
    * Links transcript text to media playback, potentially with waveform interaction.
    * Allows saving a copy of the media-based transcript as a standard document in the Fieldnotes section.
* **Associated Backend**:
    * `projectview/transcription_commands.rs`:
        * `load_transcript_json`: Loads `.json` files containing media-based transcript data (segments, speaker info).
        * `save_transcript_json`: Saves the edited transcript data back to its `.json` file.
        * `trim_media`: For creating media clips.
        * `save_speaker_config`: To save speaker configurations related to a media file.
    * `RichTextPreview.svelte`'s "Convert Media Transcript to Document" feature would invoke commands in `projectview/document_commands.rs` to create a new Lexical JSON document in the `Documents/` folder.
    * Transcription and Diarization (initiated from this view):
        * Local: `projectview/local_handler/transcription.rs` (`run_transcription`, `cancel_transcription`), also uses `local_handler/transcription.rs` for FFmpeg and diarize-cli sidecar operations.
        

---

### 3. Core Services & Stores (`src/lib/services/`, `src/lib/stores/`, `src/lib/workers/`)

This section outlines key JavaScript/TypeScript modules that provide core frontend logic, state management, and background processing.

* **Frontend Components/Modules**:
    * `services/projectService.js`: This service is the **primary communication bridge** between the SvelteKit frontend components and the Rust backend. It encapsulates almost all `invoke` calls to Tauri's backend command handlers. Its critical role is to abstract these backend interactions for the UI components, managing the flow of data and responses for a wide range of application functionalities. This includes, but is not limited to, project creation/loading, media import, initiating transcription requests, document saving/loading, and handling PDF/image annotation operations. It essentially centralizes the frontend's side of the frontend-backend contract.
    * `services/configureActions.js`: Contains functions for managing application configuration (models, themes, download location) and initiating actions like transcript export.
    * `stores/projectStore.js`: Manages the core reactive state for the currently loaded project and general application UI. Its responsibilities include:
        *   **Project Configuration**: Storing the project name, XML path, and base directory.
        *   **File System Representation**: Holding the hierarchical list of project files and assets (`files`, `documentFiles`, `tableFiles`, `imageFiles`, `importedTranscriptFiles`, `documentMetadataFiles`).
        *   **Document and Note Management**: Tracking the state for editing non-transcript text documents, PDFs (including annotations), imported transcripts (as documents), and media-specific notes. This includes selected paths, current content, dirty states, loading states, and active editor references for these items.
        *   **General UI State**: Managing global UI states such as overall loading indicators (`isLoading`), general error messages (`error`), status messages (`statusMessage`), autosave preference (`autosaveEnabled`), and the state for UI prompts like unsaved changes (for non-transcript items) and file conversion confirmations.
        *   It no longer manages the detailed state for media-based transcript editing, media player control, or the transcription process itself; these responsibilities have been moved to `transcriptStore.js`. This Svelte store allows different components to react to changes in project data dynamically.
    * `stores/transcriptStore.js`: This Svelte store is dedicated to managing all state related to media transcription and playback within the main "Transcriptions" view. Its key responsibilities include:
        *   **Transcript Segments**: Holding and managing the array of transcript segments, including their text content, start/end times, and speaker assignments. It supports operations like loading segments, updating individual segments, inserting new segments, and deleting segments.
        *   **Undo/Redo**: Manages undo and redo stacks specifically for changes made to transcript segments.
        *   **Media Player State**: Controls the state of the main media player in the transcriptions view, such as current playback time, total duration, play/pause status, and the associated audio buffer for waveform display.
        *   **Selected Media**: Tracks the currently selected media file (`selectedMediaFile`) that is active in the transcriptions view.
        *   **Speaker Configuration**: Manages the speaker count and their names for the active media's transcript.
        *   **Transcription Process State**: Handles state related to the transcription process itself, including the selected transcription model (`selectedModelName`), selected language (`selectedLanguage`), whether a transcription is currently in progress (`isTranscribing`), the progress of an ongoing transcription (`transcriptionProgress` including percent and message), the unique ID of the current transcription job (`transcriptionJobId`), and the visibility of the transcription confirmation modal (`showTranscribeModal`).
        *   **Dirty State**: Tracks whether the current transcript has unsaved changes (`transcriptDirty`).
    * `stores/themeStore.js`: Manages theme (light/dark/system) preferences, persisting them and applying them across the application.

#### Frontend Workers
    * #### `lib/workers/pdfAnnotationMatcher.worker.js` (PDF Annotation Text Matching)
        *   **Purpose**: This web worker is responsible for accurately locating text-based PDF annotations within the rendered PDF page content. When a user creates a text highlight annotation in a PDF, this worker helps ensure it can be reliably found and re-highlighted later.
        *   **How it Works**: It operates in a separate thread to avoid blocking the main UI. It receives the annotation's exact text, the text immediately preceding (prefix) and following (suffix) the annotation for contextual accuracy, and the expected occurrence number of that text on the page (e.g., the 3rd time "sample text" appears). It then processes the PDF page's full text content to find a precise match for this contextualized text.
        *   **Importance**: This is crucial for the persistence and reliability of text annotations. PDF rendering can be complex, and exact coordinates might not always be stable across sessions or minor rendering changes. Anchoring annotations to the textual content, with context, provides a more robust way to re-apply visual highlights.
        *   **Matching Strategy**: The worker uses regular expressions to find matches. It first attempts a precise match using the provided text and its surrounding context (prefix and suffix). If this fails, it falls back to a simpler regex using just the annotation text and occurrence, which can help in cases where the immediate context might have subtle variations.

#### Key Frontend Libraries & Technologies
*   **Lexical**: Core rich-text editing framework. Used extensively for the main document editor (for `.txt`, `.md`, `.rtf` imports, and new notes) and the interactive transcript editor. Provides a highly extensible and customizable editing experience.
*   **PDF.js (Mozilla)**: The primary library for rendering and displaying PDF documents directly in the browser/webview. `PDFViewerPanel.svelte` relies on this to show PDF content and enable interactions like text selection for annotations.
*   **Tabulator.js**: Used to display and interact with tabular data. `TableViewerPanel.svelte` utilizes Tabulator to render `.csv` and `.xlsx` files with features like sorting, filtering, and searching within the table.
*   **OpenSeadragon**: A library for deep zooming and smooth panning of high-resolution images. This is used in `ImageViewerPanel.svelte` to provide a good user experience when viewing detailed images.
*   **Annotorious**: Used for image annotation functionalities within `ImageViewerPanel.svelte`, allowing users to create, display, and save annotations on image files. Annotations are managed via backend commands in `projectview/image_handler.rs` and stored in `harvey_annotations.sqlite`.
*   **Svelte Stores**: Svelte's built-in mechanism for reactive state management. `projectStore.js` and `themeStore.js` are prime examples, allowing data to be shared and synchronized across different Svelte components efficiently.
*   **Tailwind CSS**: A utility-first CSS framework used for styling the user interface. Provides a set of pre-designed utility classes to build custom designs rapidly. Its presence is indicated by `tailwind.config.js`.

* **Functionality Overview**:
    * `projectService.js`: Centralizes frontend-to-backend communication via Tauri `invoke` calls.
    * `configureActions.js`: Handles settings UI logic and actions like export.
    * `projectStore.js`: Centralized state for the active project view.
    * `themeStore.js`: Handles theme state and persistence.
* **Associated Backend**:
    * `configureActions.js`:
        * Model/API/Theme/Download Location: `welcome/commands.rs`.
        * Transcript Export: `projectview/export_handler.rs` (`export_transcript_to_docx`).
    * `themeStore.js`: `welcome/commands.rs` (`get_theme_preference`, `set_theme_preference`).
    * `projectStore.js`: Populated and updated by data flowing from almost all backend modules via `projectService.js`.

---

### 4. Routing (`src/routes/`)

* **Frontend Components**:
    * `+layout.svelte`: Root layout for the SvelteKit application.
    * `+page.svelte`: The main page, typically rendering the `WelcomeScreen.svelte`.
    * `projectview/+page.svelte`: SvelteKit page component for the project view route.
    * `projectview/ProjectView.svelte`: The main Svelte component for the project interface.
* **Functionality**: Handles application routing and layout.
* **Associated Backend**:
    * `+layout.svelte` (on mount): Could call `welcome/commands.rs` (`get_theme_preference`) to initialize the theme.
    * `+page.svelte` (Welcome Screen): Uses `welcome/commands.rs` for project listing, creation, etc.
    * `projectview/ProjectView.svelte` (on mount): Uses `projectview/core_commands.rs` (`load_project_data`) to load the selected project's data.

---

### 5. Backend Entry & Core Structure (`src-tauri/src/`)

* **Backend Modules**:
    * `lib.rs`: This is the main entry point for the Rust backend when the Tauri application starts. Its key responsibilities include:
        *   Initializing the Tauri application environment.
        *   Managing global, shared states accessible across the backend, such as `DownloadCancellationState` (for cancellable model downloads) and `TranscriptionCancellationState` (for cancellable transcription tasks). These states typically use `Arc<DashMap<String, Arc<AtomicBool>>>` for thread-safe, mutable access to cancellation flags.
        *   Initializing essential Tauri plugins:
            *   `tauri-plugin-dialog`: For native system dialogs (file open/save, confirmations).
            *   `tauri-plugin-fs`: For filesystem access (reading/writing files, directory operations).
            *   `tauri-plugin-shell`: For executing external shell commands (e.g., running sidecar processes for AI models).
            *   `tauri-plugin-opener`: For opening files or URLs with their default system applications.
        *   Registering all backend command handlers using `tauri::generate_handler!`. This makes Rust functions callable from the SvelteKit frontend via `invoke()`. The list includes commands from `welcome::commands` and the various `projectview::*::commands` modules.
        *   Calling `init_projectview_db()` to initialize or migrate the SQLite database (`harvey_annotations.sqlite`) used for storing annotations, ensuring it's ready before the application fully loads.
    * `main.rs`: Entry point for the desktop application, which simply calls the `run()` function from `harvey_1_lib` (i.e., `lib.rs`).
    * `build.rs`: Build script, primarily used for tasks like ensuring sidecar executable files for local AI models are correctly packaged with the application.
    * `projectview/shared_types.rs`: Defines common Rust structs (e.g., XML structures for `.harvey` files like `ProjectXml`, `FileEntry`, `DocumentEntryXml`, etc.) and constants used across the `projectview` modules. These types ensure data consistency.
    * `projectview/shared_utils.rs`: Contains reusable helper functions for tasks like XML processing for `.harvey` files, path manipulation, and ensuring project asset directories exist.
    * #### `db_handler.rs` (SQLite Database for Annotations)
        *   This module manages the `harvey_annotations.sqlite` database, located in the application's user configuration directory.
        *   **Primary Table (`pdf_annotations`) Structure**:
            *   `id`: INTEGER PRIMARY KEY AUTOINCREMENT
            *   `pdf_document_path`: TEXT NOT NULL (Path to the associated document file, e.g., PDF, image)
            *   `annotations_json`: TEXT NOT NULL (Stores annotation data as a JSON string)
            *   `document_type`: TEXT NOT NULL (e.g., 'pdf', 'image'; vital for distinguishing annotation targets)
            *   `created_at`: TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            *   `updated_at`: TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        *   The pair `(pdf_document_path, document_type)` acts as a unique key.
        *   **Key Functionalities**: Database initialization (including schema migration for `document_type`), loading, saving (insert/update), deleting, and renaming annotation entries (e.g., when a source file path changes).
        *   This SQLite database enables persistent storage of annotations, independent of the main `.harvey` project file, and links them to specific file paths and types.
* **Overall Backend Functionality**:
    * Initializes the Tauri application, sets up windows, and manages the communication bridge between the SvelteKit frontend and Rust. The backend commands are extensive, covering project lifecycle management (via `welcome::commands`) to detailed asset manipulations (media, documents, transcripts, annotations, tables, images) and external process handling (downloads, local AI tasks) (via `projectview::*` modules).
    * Defines shared data structures (like those in `shared_types.rs`) and utility functions (in `shared_utils.rs`) crucial for robust backend operations.
* **Capabilities (`src-tauri/capabilities/default.json`)**: This file defines the permissions for the Tauri application, granting necessary access to the file system (e.g., `fs:allow-*`), native dialogs (`dialog:allow-*`), execution of sidecar processes (`shell:allow-execute`, `shell:allow-spawn`), and window management.

---