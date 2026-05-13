# Harvey

## Overview

Harvey is a desktop application for qualitative researchers and anyone working with multimedia content. It streamlines the research workflow by integrating a powerful suite of tools into a single, cohesive environment:

* **Transcribe** audio and video with local AI models.
* **Translate** transcripts from one language to another.
* **Edit** and **annotate** transcripts, documents, PDFs, and images.
* **Manage** all your project files in one place.

Harvey is named in honor of Sociologist [**Harvey Sacks**](https://en.wikipedia.org/wiki/Harvey_Sacks) (1935-1975), whose foundational work revolutionized social research methods.

Visit the [Harvey Website](https://ethnomethodology.github.io/harvey/).

### Privacy First

Privacy is the core of Harvey's design. All core AI functionalities, like transcription and diarization (speaker identification), run **100% locally** on your computer. You download the AI models once and can use them forever offline. Your data never leaves your device.


### Developed with AI

Project Harvey has been developed entirely using Vibe Coding to assess the viability of AI-assisted programming in research software development. From its inception, we have developed this application using AI tools such as Gemini, Google AI Studio, and Google Jules. We believe this AI-assisted approach can make development more accessible and efficient, and we encourage contributors to explore and adopt similar methods. Read about [how and why we developed Harvey](https://blogs.lse.ac.uk/impactofsocialsciences/2026/05/05/vibe-coding-for-qualitative-researchers-can-ai-really-build-our-research-tools/). (LSE Blog Post)



## Key Features

* **Offline AI Processing**:
    * **Transcription & Diarization**: Use state-of-the-art speech-to-text models (e.g., OpenAI's Whisper) locally to transcribe your audio and video files for complete data privacy.
    * **Translation**: Translate generated transcripts between multiple languages using local AI models (e.g., NLLB, Helsinki-NLP).
* **Interactive Transcript Editor**:
    * Rich-text editor linked directly to media playback.
    * Adjust timestamps and manage speaker labels with ease.
* **Comprehensive Data Management**:
    * **Audios and Videos**: Import, play, and trim your audio and video files.
    * **Documents**: Create, edit, and annotate rich-text documents and field notes.
    * **PDFs**: View and annotate PDF documents with robust text-based tools.
    * **Images**: Import, view, and annotate images with deep-zoom support.
    * **Tables**: View and edit CSV and XLSX files with support for nested sub-documents.
* **Project & Asset Organization**:
    * Manage all your media files, documents, and transcripts across multiple projects.
* **User-Friendly & Customizable**:
    * **Cross-Platform**: Works seamlessly on Windows, macOS, and Linux.
    * **Configurable**: Customize transcription models, application themes, and model download locations.
    * **Data Export**: Export transcripts to common formats like DOCX and CSV.

## Installation Guide

### macOS
1. Download the `.dmg` file from [GitHub Releases](https://github.com/Ethnomethodology/harvey/releases).
    * For Intel Macs, download the file with `x64` in the name.
    * For Apple Silicon (M1, M2, etc.) Macs, download the file with `aarch64` in the name.
2. Open the downloaded `.dmg` file.
3. Drag and drop `harvey.app` to your `/Applications` folder.
4. Close the `.dmg` file window.
5. Open the Terminal application and run the following commands to bypass macOS Gatekeeper:
   ```bash
   cd /Applications
   sudo xattr -dr com.apple.quarantine harvey.app
   ```

### Windows
1. Download the `.exe` installer file from [GitHub Releases](https://github.com/Ethnomethodology/harvey/releases).
2. Run the downloaded `.exe` file.
3. Ensure you have administrative privileges to complete the installation.

### Linux

Pre-compiled binaries for Linux are coming soon. In the meantime, you can use our automated bootstrapper to set up your environment and build Harvey from source:

```bash
curl -sSL https://raw.githubusercontent.com/Ethnomethodology/harvey/main/scripts/bootstrap.sh | bash
```

Alternatively, you can clone the repository and run the script manually:

```bash
git clone https://github.com/Ethnomethodology/harvey.git
cd harvey && bash scripts/bootstrap.sh
```

For more detailed manual setup instructions, see our [Setup Guide](Docs/SETUP_GUIDE.md).

## Contributing to Harvey

We warmly welcome contributions to Project Harvey from everyone in the open-source community! Whether you're taking your first steps into coding, looking to experiment with AI-assisted development, or a qualitative researcher wanting to shape the tools you use, your input is valuable.

Project Harvey is particularly interested in fostering a collaborative environment that embraces innovative coding paradigms. We encourage contributions from users of "vibe coding" and "agentic coding" platforms (such as those being explored with Google Jules or GitHub Copilot).


### Getting Started for Developers & AI Agents
Harvey employs a strict **"Visual First" documentation standard**.

1. **Setup Environment**: See [Docs/SETUP_GUIDE.md](Docs/SETUP_GUIDE.md) to get your local SvelteKit + Tauri + Rust environment running.
2. **Understand the Architecture**: Read [Docs/PROJECT_DESCRIPTION.md](Docs/PROJECT_DESCRIPTION.md) for a high-level overview. For database structure, see [Docs/DATABASE_SCHEMA.md](Docs/DATABASE_SCHEMA.md).
3. **Follow the Rules**: Before contributing, you **must** read [CONTRIBUTING.md](CONTRIBUTING.md) to understand our documentation templates.
4. **For AI Assistants**: If you are using an AI agent (like Jules) to modify code, ensure it reads [AGENTS.md](AGENTS.md) first to understand its binding constraints regarding documentation updates and styling.

*Note: Almost every subdirectory in `src/lib/` and `src-tauri/src/` contains its own `README.md` with Mermaid diagrams detailing its specific component architecture and data flow. Always consult these colocated READMEs before modifying code!*

## Help Center Architecture

The Help Center content is maintained in a **Single Source of Truth** architecture to ensure consistency between the website and the desktop application.

*   **Source**: All help articles are authored as Markdown files in `website/src/content/help`.
*   **Synchronization**: A custom script, `scripts/sync-help.js`, automatically copies these files from the website directory to the desktop app's directory (`src/content/help`).
*   **Automation**: This synchronization script runs automatically before every `dev` and `build` command (via `predev` and `prebuild` scripts in `package.json`), ensuring the desktop app always contains the latest documentation.
*   **Internal Linking**: To ensure compatibility between the website (SvelteKit) and the desktop app (Custom Modal), use **relative links** for internal navigation (e.g., `[Link](slug)` or `[Link](page-id)`). Do not use absolute paths (e.g., `/help/slug`).

## License

This project is licensed under the [MIT License](LICENSE.md).
