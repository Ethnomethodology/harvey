# Project Harvey 1.0: Module Breakdown

This document details the frontend SvelteKit components and their associated Rust backend modules for the Harvey 1.0 application.

---

## 1. Welcome Screen & Configuration (`src/lib/components/welcome/`)

* **Frontend Components**:
    * `WelcomeScreen.svelte`: Main entry point for project management and application configuration.
    * `ProjectList.svelte` & `ProjectItem.svelte`: Display the list of recent projects.
    * `Configure.svelte`: UI for application settings (download locations, API keys, models, theme).
    * `RenameModal.svelte`: Modal for renaming projects.
    * `actions.js`: Contains client-side logic for invoking backend commands related to this section.
* **Functionality**:
    * Manages the creation, opening, renaming, listing, importing, and deletion of transcription projects.
    * Handles global application configuration, including download paths for transcription models, cloud API (Gemini) keys, theme preferences, and management of downloaded models.
* **Associated Backend (`src-tauri/src/welcome/`)**:
    * `commands.rs`:
        * Project actions: `load_recent_projects`, `create_project`, `rename_project`, `remove_project_from_list`, `open_project`, `import_project`, `delete_project`, `locate_in_finder`.
        * Configuration: `ensure_directory`, `save_download_location`, `get_download_location`, `get_downloaded_models`, `delete_model`, `download_model_command`, `cancel_download_command`, `change_download_location_and_move_models`, `get_cloud_config`, `save_cloud_config`, `get_theme_preference`, `set_theme_preference`.
    * `config.rs`: Handles the reading and writing of the global `config.xml` file which stores recent projects and settings.

---

## 2. Project View (`src/routes/projectview/` & `src/lib/components/projectview/`)

This is the main multi-tab interface for working within a project.

#### Overall Structure & Navigation

* **Frontend Components**:
    * `ProjectView.svelte` (in `src/routes/projectview/`): The root component for the project workspace, likely managing the "Transcriptions" and "Fieldnotes" tabs.
    * `BottomBar.svelte` (in `src/lib/components/projectview/shared/`): A shared component for displaying status messages or common actions across project views.
* **Functionality**:
    * Provides the main tabbed interface for "Transcriptions" (media-based) and "Fieldnotes" (asset management).
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
        * Cloud Transcription: `projectview/cloud_handler/cloud_transcribe.rs` (`run_cloud_transcription`, `cancel_cloud_transcription`).
    * Other modals primarily manage frontend state or simple confirmations.

---

#### Fieldnotes View (`src/lib/components/projectview/notes/`)

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
    * `MediaPlayer.svelte`: Handles audio/video playback.
    * `InteractiveWaveform.svelte`: Displays an interactive audio waveform.
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
        * Cloud: `projectview/cloud_handler/cloud_transcribe.rs` (`run_cloud_transcription`, `cancel_cloud_transcription`).

---

### 3. Core Services & Stores (`src/lib/services/` & `src/lib/stores/`)

* **Frontend Components/Modules**:
    * `services/projectService.js`: Acts as a central hub for frontend components to make `invoke` calls to various backend Rust commands. It doesn't have one specific backend module but interacts with nearly all of them in `src-tauri/src/projectview/` and `src-tauri/src/welcome/`.
    * `services/configureActions.js`: Contains functions for managing application configuration (models, API keys, themes, download location) and initiating actions like transcript export.
    * `stores/projectStore.js`: Manages the reactive state for the currently loaded project, including file lists, editor states, selections, and UI states.
    * `stores/themeStore.js`: Manages theme (light/dark/system) preferences.
* **Functionality**:
    * `projectService.js`: Abstracts backend communication.
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
    * `lib.rs`: Main Tauri setup, registers all invoke handlers from `welcome::commands` and `projectview::*::commands`. Manages global states like `DownloadCancellationState` and `TranscriptionCancellationState`.
    * `main.rs`: Entry point for the desktop application, calls `harvey_1_lib::run()`.
    * `build.rs`: Build script, handles tasks like copying sidecar model files.
    * `projectview/shared_types.rs`: Defines common Rust structs (e.g., XML structures like `ProjectXml`, `FileEntry`, `DocumentEntryXml`, `TableEntryXml`, `ImageEntryXml`, `ImportedTranscriptEntryXml`, `PdfAnnotationEntryXml`) and constants used across the `projectview` modules.
    * `projectview/shared_utils.rs`: Contains reusable helper functions for tasks like XML saving, path manipulation, and ensuring asset directories exist.
* **Functionality**:
    * Initializes the Tauri application, sets up windows, and manages communication between the SvelteKit frontend and the Rust backend.
    * Defines shared data structures and utility functions crucial for backend operations.
* **Capabilities (`src-tauri/capabilities/default.json`)**: Defines the permissions for the Tauri application, such as file system access (`fs:allow-*`), dialogs (`dialog:allow-*`), shell commands for sidecars (`shell:allow-execute`, `shell:allow-spawn`), and window management.

---