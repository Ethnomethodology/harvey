# Harvey

## Overview/Core Purpose

Harvey is a comprehensive desktop application designed for researchers, journalists, and individuals who work with multimedia content. The application is particularly aimed at academic and qualitative researchers. It provides a robust suite of tools for managing projects, transcribing audio and video files using both local AI models and cloud-based services (e.g., Google Gemini), editing transcripts with a feature-rich text editor, and handling various related documents and media such as PDFs, text files, images, and tables. The application aims to streamline the workflow of analyzing qualitative data by integrating these functionalities into a cohesive, cross-platform experience (Windows, macOS, Linux) built with Tauri and Rust for performance and reliability.

Harvey is designed with privacy as a priority. Core AI functionalities like transcription and diarization run locally on the user's computer. Users download the necessary AI models once and can then use them repeatedly offline, ensuring data remains on their device.

It's an application built by researchers, for researchers.

## Key Features

*   **Comprehensive Project Management**: Create, open, import, and manage multimedia research projects using `.harvey` project files.
*   **Media Transcription (Dual Mode)**:
    *   **Local AI**: Perform transcription and diarization using downloaded speech-to-text models (e.g., Whisper variants) for offline processing, ensuring data privacy.
    *   **Cloud-Powered**: Leverage cloud-based transcription services (e.g., Google Gemini) for high-accuracy transcription with an internet connection.
*   **Interactive Transcript Editor**: Edit and refine transcripts with a rich-text editor (Lexical-based) linked to media playback, including timestamp adjustment and speaker labeling.
*   **Versatile Document Handling**:
    *   **Rich Text Documents**: Create and edit notes and documents with formatting, tables, and lists.
    *   **PDF Viewing & Annotation**: Open and view PDF documents, with robust text-based annotation capabilities.
    *   **Table Viewing**: Import and view CSV and XLSX files with interactive features.
    *   **Image Handling & Annotation**: Import, view (OpenSeadragon), and annotate (Annotorious) various image formats.
*   **Asset Management**: Organize various project assets including media files, transcripts, documents, images, and tables.
*   **Configurable Settings**: Customize application behavior, including transcription model selection, API key management, and UI themes.
*   **Cross-Platform**: Runs on Windows, macOS, and Linux.
*   **Data Export**: Export transcripts to common formats (e.g., DOCX).
*   **Background Processing**: Handles intensive tasks like model downloads and transcriptions in the background.

## Contributing to Harvey

We warmly welcome contributions to Project Harvey from everyone in the open-source community! Whether you're taking your first steps into coding, looking to experiment with AI-assisted development, or a qualitative researcher wanting to shape the tools you use, your input is valuable.

Project Harvey is particularly interested in fostering a collaborative environment that embraces innovative coding paradigms. We encourage contributions from users of "vibe coding" and "agentic coding" platforms (such as those being explored with Google Jules, for example). We see these approaches as exciting ways to build software:

*   **Vibe Coding**: This is about setting the general direction or 'vibe' for a feature. You might have a clear outcome in mind but perhaps not all the technical steps. AI tools can help you flesh out the specifics, and you refine the code iteratively. It's a partnership where you guide the AI to achieve your vision.
*   **Agentic Coding**: Here, the AI acts more like an autonomous agent. It can take on more complex tasks, almost like a junior developer, capable of planning and executing coding steps with a degree of independence based on your higher-level goals.

From its inception, Project Harvey has been significantly developed using AI tools, including Gemini, Google AI Studio, and explorations with Google Jules. We believe this AI-assisted approach can make development more accessible and efficient, and we encourage contributors to explore and adopt similar methods.

We are always keen to hear from our users and contributors about:

*   Issues you might be facing with Harvey.
*   What aspects of the application you find most useful.
*   Features you would love to see added.
*   Any help or guidance you might need regarding contributing to the Harvey Project.

Your feedback and contributions are crucial in making Harvey a better tool for everyone.

To get started, please refer to our documentation:

*   To get your local copy of Harvey up and running, see our: [Setup Guide (Docs/SETUP_GUIDE.md)](Docs/SETUP_GUIDE.md)
*   To understand how Harvey is structured and how its different parts work, refer to the: [Project Description (Docs/PROJECT_DESCRIPTION.md)](Docs/PROJECT_DESCRIPTION.md)
*   For guidance on making contributions, especially using AI-assisted coding tools like Google Jules, check out our: [Developer's Contribution Guide (Docs/DEVELOPERS_GUIDE.md)](Docs/DEVELOPERS_GUIDE.md)

## Technology Stack

*   **Core Framework**: [Tauri](https://tauri.app/) (v2.0, Rust backend, webview frontend)
*   **Backend Language**: [Rust](https://www.rust-lang.org/)
*   **Frontend Framework**: [SvelteKit](https://kit.svelte.dev/)
*   **Frontend Languages**: JavaScript (SvelteKit), HTML, CSS
*   **Text Editor**: [Lexical](https://lexical.dev/)
*   **PDF Viewing**: [PDF.js](https://mozilla.github.io/pdf.js/)
*   **Table Display**: [Tabulator](https://tabulator.info/)
*   **Image Viewing/Annotation**: OpenSeadragon (viewing), Annotorious (annotations)
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