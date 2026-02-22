---
id: config-transcription
label: Transcription Engine
sidebarId: configure
order: 12
---

### Transcription Engine

Harvey provides two distinct engines for local transcription. You can switch between them in the **Configure** tab under the **Transcription** section.

> **Note:** Only models belonging to the currently selected engine will be available for transcription. If you switch engines, you must download at least one compatible model for that engine.

#### 1. Whisper.cpp
*   **Recommended for macOS:** Highly optimized for Apple Silicon (M1, M2, etc.) using Core ML.
*   **Performance:** Extremely fast and efficient on Mac hardware.

#### 2. Faster-Whisper
*   **Recommended for Windows (CPU):** Uses the CTranslate2 backend, which is often significantly faster than whisper.cpp on Intel and AMD CPUs.
*   **Performance:** Provides an excellent balance of speed and accuracy for systems without high-end GPUs.

### Managing Models
For either engine, you can download various model sizes (Tiny, Base, Small, Medium, Large). Larger models provide higher accuracy but require more RAM and processing power.
