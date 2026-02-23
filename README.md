# Harvey

## Overview

Harvey is a desktop application for qualitative researchers and anyone working with multimedia content. It streamlines the research workflow by integrating a powerful suite of tools into a single, cohesive environment:

* **Transcribe** audio and video with local AI models.
* **Translate** transcripts into English.
* **Edit** and **annotate** transcripts, documents, PDFs, and images.
* **Manage** all your project files in one place.

Harvey is named in honor of Sociologist [**Harvey Sacks**](https://en.wikipedia.org/wiki/Harvey_Sacks){:target="_blank"} (1935-1975), whose foundational work revolutionized social research methods.

Visit the [Harvey Website](https://ethnomethodology.github.io/harvey/){:target="_blank"}.

### Privacy First

Privacy is the core of Harvey's design. All core AI functionalities, like transcription and diarization (speaker identification), run **100% locally** on your computer. You download the AI models once and can use them forever offline. Your data never leaves your device.

> It's an application built *by researchers, for researchers*.

## Key Features

* **Offline AI Processing**:
    * **Transcription & Diarization**: Use state-of-the-art speech-to-text models (e.g., OpenAI's Whisper) locally to transcribe your audio and video files for complete data privacy.
    * **Translation**: Translate generated transcripts to English using local AI models.
* **Interactive Transcript Editor**:
    * Rich-text editor linked directly to media playback.
    * Adjust timestamps and manage speaker labels with ease.
* **Comprehensive Data Management**:
    * **Audios and Videos**: Import, play, and trim your audio and video files.
    * **Documents**: Create, edit, and annotate rich-text documents and field notes.
    * **PDFs**: View and annotate PDF documents with robust text-based tools.
    * **Images**: Import, view, and annotate images.
    * **Tables**: View and edit CSV and XLSX files.
* **Project & Asset Organization**:
    * Manage all your media files, documents, and transcripts across multiple projects.
* **User-Friendly & Customizable**:
    * **Cross-Platform**: Works seamlessly on Windows, macOS, and Linux.
    * **Configurable**: Customize transcription models, application themes, and more.
    * **Data Export**: Export transcripts to common formats like DOCX and CSV.

## Contributing to Harvey

We warmly welcome contributions to Project Harvey from everyone in the open-source community! Whether you're taking your first steps into coding, looking to experiment with AI-assisted development, or a qualitative researcher wanting to shape the tools you use, your input is valuable.

Project Harvey is particularly interested in fostering a collaborative environment that embraces innovative coding paradigms. We encourage contributions from users of "vibe coding" and "agentic coding" platforms (such as those being explored with Google Jules, for example). We see these approaches as exciting ways to build software:

*   **Vibe Coding**: This is about setting the general direction or 'vibe' for a feature. You might have a clear outcome in mind but perhaps not all the technical steps. AI tools can help you flesh out the specifics, and you refine the code iteratively. It's a partnership where you guide the AI to achieve your vision.
*   **Agentic Coding**: Here, the AI acts more like an autonomous agent. It can take on more complex tasks, capable of planning and executing coding steps with a degree of independence based on your higher-level goals.

From its inception, Project Harvey has been significantly developed using AI tools, including Gemini, Google AI Studio, and Google Jules. We believe this AI-assisted approach can make development more accessible and efficient, and we encourage contributors to explore and adopt similar methods.

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

## Help Center Architecture

The Help Center content is maintained in a **Single Source of Truth** architecture to ensure consistency between the website and the desktop application.

*   **Source**: All help articles are authored as Markdown files in `website/src/content/help`.
*   **Synchronization**: A custom script, `scripts/sync-help.js`, automatically copies these files from the website directory to the desktop app's directory (`src/content/help`).
*   **Automation**: This synchronization script runs automatically before every `dev` and `build` command (via `predev` and `prebuild` scripts in `package.json`), ensuring the desktop app always contains the latest documentation.
*   **Internal Linking**: To ensure compatibility between the website (SvelteKit) and the desktop app (Custom Modal), use **relative links** for internal navigation (e.g., `[Link](slug)` or `[Link](page-id)`). Do not use absolute paths (e.g., `/help/slug`).

## Documentation

For more detailed information, please refer to the guides in the `Docs/` folder, including the Developer's Guide, Setup Guide, and the full Project Description.

## Installation Guide

### macOS
1. Download the `.dmg` file from [GitHub Releases](https://github.com/Ethnomethodology/harvey/releases).
    * For Intel Macs, download the file with `x64` in the name.
    * For Apple Silicon (M1, M2, etc.) Macs, download the file with `aarch64` in the name.
2. Open the downloaded `.dmg` file.
3. Drag and drop `harvey.app` to your `/Applications` folder.
4. Close the `.dmg` file window.
5. Open the Terminal application and run the following commands:
   ```bash
   cd /Applications
   sudo xattr -dr com.apple.quarantine harvey.app
   ```

### Windows
1. Download the `.msi` installer file from [GitHub Releases](https://github.com/Ethnomethodology/harvey/releases).
2. Run the downloaded `.msi` file.
3. Ensure you have administrative privileges to complete the installation.

### Linux
Installation guide is coming soon.

## License

This project is licensed under the [MIT License](LICENSE.md).

