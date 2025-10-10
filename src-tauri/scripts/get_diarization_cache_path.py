# src-tauri/scripts/get_diarization_cache_path.py
import os
from pathlib import Path

def get_cache_path():
    """
    Prints the Hugging Face cache path.
    """
    try:
        # Respect HF_HOME environment variable if set, otherwise default
        hf_home = os.environ.get("HF_HOME", Path.home() / ".cache" / "huggingface")
        hub_path = Path(hf_home) / "hub"
        print(str(hub_path))
    except Exception as e:
        print(f"Error: Failed to determine cache path: {e}", file=sys.stderr)
        sys.exit(1)

if __name__ == "__main__":
    get_cache_path()
