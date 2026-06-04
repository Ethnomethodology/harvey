import re

with open('src-tauri/src/welcome/commands.rs', 'r') as f:
    text = f.read()

# Find the start of install_faster_whisper_dependencies_command
fw_install_match = re.search(r'(#\[command\]\s*pub async fn install_faster_whisper_dependencies_command.*?^})', text, re.MULTILINE | re.DOTALL)
if fw_install_match:
    fw_install_code = fw_install_match.group(1)
    cw_install_code = fw_install_code.replace("faster_whisper", "crisper_whisper").replace("Faster-Whisper", "Crisper-Whisper")
    # Insert cw_install_code after fw_install_code
    text = text.replace(fw_install_code, fw_install_code + "\n\n" + cw_install_code)

# Find the start of download_faster_whisper_model_command
fw_download_match = re.search(r'(// --- Transcription Model Download Command \(Faster-Whisper\) ---.*?^})', text, re.MULTILINE | re.DOTALL)
if fw_download_match:
    fw_download_code = fw_download_match.group(1)
    cw_download_code = fw_download_code.replace("faster_whisper", "crisper_whisper").replace("Faster-Whisper", "Crisper-Whisper").replace("faster-whisper", "crisper-whisper")
    text = text.replace(fw_download_code, fw_download_code + "\n\n" + cw_download_code)

with open('src-tauri/src/welcome/commands.rs', 'w') as f:
    f.write(text)
