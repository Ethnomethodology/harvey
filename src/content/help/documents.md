---
id: documents
label: Documents
sidebarId: manage-data
order: 24
---

Work with text-based assets including Lexical JSON, PDF, Markdown, and TXT files.

*   **Rich Text Editor:** Edit documents with standard formatting tools (Bold, Italic, Lists, etc.).
*   **PDF Viewer:** View and annotate PDF documents directly. Annotations are saved as metadata and can be tagged.
*   **Highlights:** Select any text to create a highlight. Assign tags to these highlights for cross-project analysis.

### Live Transcription
You can create a new document by recording audio directly through your microphone. Harvey will transcribe your speech in real-time as you speak.

For the best real-time performance and minimal latency, we recommend using smaller or optimized models:

#### Based on your Engine:
*   **Whisper.cpp (macOS):** Use the `ggml-large-v3-turbo` model for a great balance of speed and accuracy, or `ggml-tiny.en` for the lowest possible latency.
*   **Faster-Whisper (Windows):** Use `distil-large-v3` for high-speed, high-accuracy live text, or `tiny.en` for maximum responsiveness on older hardware.
