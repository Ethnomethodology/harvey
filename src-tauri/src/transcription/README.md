# Module: Transcription (`src-tauri/src/transcription`)

**Purpose:** Manages the integration, execution, and data parsing of external machine learning models (Whisper.cpp and Faster-Whisper) for both batch file transcription and real-time live transcription.

## Architecture & Data Flow
*Use a Mermaid flowchart to map how frontend calls route through the commands down to the handlers and external systems.*
```mermaid
flowchart LR
    Frontend([Svelte Frontend]) -. "invoke('start_transcription')" .-> Commands[transcription_commands.rs in projectview]
    Commands --> TranscriptionModule[transcription/mod.rs]

    TranscriptionModule --> WhisperCpp[whisper_cpp.rs]
    TranscriptionModule --> FasterWhisper[faster_whisper.rs]
    TranscriptionModule --> FasterWhisperLive[faster_whisper_live.rs]

    FasterWhisper --> PythonEngine[python_engine.rs (Subprocess wrapper)]
    FasterWhisperLive --> PythonEngine

    WhisperCpp --> CLI[Whisper.cpp Binary]
    PythonEngine --> PyScript[Python Inference Scripts]
```

## Internal Handlers
*   **`faster_whisper.rs`**: Handles building command-line arguments and managing standard I/O for batch audio processing via the Faster-Whisper Python library. Parses stdout for progress updates.
*   **`faster_whisper_live.rs`**: Manages continuous, streaming audio input via Python subprocesses. Includes complex logic for intercepting partial vs. final segment outputs and emitting Tauri events back to the frontend in real-time.
*   **`whisper_cpp.rs`**: Handles invoking compiled Whisper.cpp C++ binaries directly for batch processing, offering an alternative engine with different performance characteristics.
*   **`python_engine.rs`**: A generalized utility module for safely spawning, monitoring, and killing isolated Python subprocesses (using paths configured during the Welcome setup).

## Managed State & Concurrency
*   **Live Transcription States**: Real-time transcription relies heavily on `Arc<Mutex<Option<Child>>>` or similar structures stored globally in Tauri's `AppHandle` to ensure that running Python inference processes can be safely aborted or interrupted by the user across different application threads.

## Expected Errors
*   **`IOError`**: Fails if the Python executable path is invalid or the Whisper binary is missing.
*   **`SubprocessError`**: Returns a serialized string error if the Python inference script crashes or returns a non-zero exit code (e.g., out of memory/VRAM).
*   **`ParsingError`**: Fails if the `stdout` from the model does not conform to the expected format (e.g., `[00:00.000 --> 00:05.000] text`).