# Project Harvey 1.0

## Overview/Core Purpose

Project Harvey 1.0 is a comprehensive desktop application designed for researchers, journalists, and individuals who work with multimedia content. It provides a robust suite of tools for managing projects, transcribing audio and video files using both local AI models and cloud-based services (e.g., Google Gemini), editing transcripts with a feature-rich text editor, and handling various related documents and media such as PDFs, text files, images, and tables. The application aims to streamline the workflow of analyzing qualitative data by integrating these functionalities into a cohesive, cross-platform experience (Windows, macOS, Linux) built with Tauri and Rust for performance and reliability.

## Key Features

*   **Comprehensive Project Management**: Create, open, import, and manage multimedia research projects using `.harvey` project files.
*   **Media Transcription (Dual Mode)**:
    *   **Local AI**: Perform transcription and diarization using downloaded speech-to-text models (e.g., Whisper variants) for offline processing.
    *   **Cloud-Powered**: Leverage cloud-based transcription services (e.g., Google Gemini) for high-accuracy transcription with an internet connection.
*   **Interactive Transcript Editor**: Edit and refine transcripts with a rich-text editor (Lexical-based) linked to media playback, including timestamp adjustment and speaker labeling.
*   **Versatile Document Handling**:
    *   **Rich Text Documents**: Create and edit notes and documents with formatting, tables, and lists.
    *   **PDF Viewing & Annotation**: Open and view PDF documents, with robust text-based annotation capabilities.
    *   **Table Viewing**: Import and view CSV and XLSX files with interactive features.
    *   **Image Handling & Annotation**: Import, view, and annotate various image formats.
*   **Asset Management**: Organize various project assets including media files, transcripts, documents, images, and tables.
*   **Configurable Settings**: Customize application behavior, including transcription model selection, API key management, and UI themes.
*   **Cross-Platform**: Runs on Windows, macOS, and Linux.
*   **Data Export**: Export transcripts to common formats (e.g., DOCX).
*   **Background Processing**: Handles intensive tasks like model downloads and transcriptions in the background.

## Technology Stack

*   **Core Framework**: [Tauri](https://tauri.app/) (v2.0, Rust backend, webview frontend)
*   **Backend Language**: [Rust](https://www.rust-lang.org/)
*   **Frontend Framework**: [SvelteKit](https://kit.svelte.dev/)
*   **Frontend Languages**: JavaScript (SvelteKit), HTML, CSS
*   **Text Editor**: [Lexical](https://lexical.dev/)
*   **PDF Viewing**: [PDF.js](https://mozilla.github.io/pdf.js/)
*   **Table Display**: [Tabulator](https://tabulator.info/)
*   **Image Viewing/Manipulation**: OpenSeadragon
*   **Annotations Database**: SQLite (managed via `rusqlite` in Rust)
*   **UI Styling**: Tailwind CSS
*   **Sidecar Execution**: For local AI models (e.g., Whisper, Diarization tools)

## Project Organization

Project Harvey is built using Tauri, with a Rust backend and a SvelteKit frontend.

*   **Frontend**: Developed with SvelteKit, managing the user interface and interactions. UI components trigger actions that are often passed to `projectService.js`, which then communicates with the Rust backend via Tauri's `invoke` mechanism.
*   **Backend**: Written in Rust, it handles core logic, file system operations, project data management, interaction with the annotations database, and execution of local AI models.
*   **Data Storage**:
    *   **Project Files (`.harvey`)**: XML-based files defining project structure and metadata.
    *   **Application Configuration (`config.xml`)**: Stores global settings like API keys and theme preferences.
    *   **Annotations (`harvey_annotations.sqlite`)**: An SQLite database for storing annotations for PDFs and images.
    *   **Project Assets**: Media files, documents, and other assets are stored within the project's dedicated folder.

## Documentation

For more detailed information, please refer to the guides in the `Docs/` folder, including the Developer's Guide, Setup Guide, and the full Project Description.