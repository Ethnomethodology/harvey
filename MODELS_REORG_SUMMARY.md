# Models Directory Reorganization

The `models` directory has been reorganized to better structure transcription and translation models.

## New Structure
- **Transcription Models:** `models/transcription/whisper-cpp/`
- **Translation Models:** `models/translation/helsinki-nlp/`

## Changes
- **Downloads:** New models will be downloaded to these specific subdirectories.
- **Usage:** The application now looks for models in these new locations first.
- **Legacy Support:** The application includes fallback logic to find models in the old location (root of `models` directory) if they haven't been moved, ensuring existing setups continue to work.
- **Movement:** When changing the download location via the settings, models will be moved to the new structure.
- **Deletion:** Deleting a model will correctly remove it from the new subdirectory.

## Modified Files
- `src-tauri/src/welcome/commands.rs`: Updated download, list, delete, and move commands.
- `src-tauri/src/projectview/translation_commands.rs`: Updated translation process to locate models.
- `src-tauri/src/projectview/transcription_commands.rs`: Updated transcription process to locate models.
