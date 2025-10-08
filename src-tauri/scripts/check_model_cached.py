# src-tauri/scripts/check_model_cached.py
import sys
from huggingface_hub import hf_hub_download
from huggingface_hub.utils import HfHubHTTPError

def check_model():
    try:
        # This will raise an HfHubHTTPError if the file is not found locally.
        hf_hub_download(
            repo_id="pyannote/speaker-diarization-3.1",
            filename="config.yaml",
            local_files_only=True
        )
        print("cached")
    except HfHubHTTPError:
        print("not_cached")
    except Exception:
        print("not_cached")

if __name__ == "__main__":
    check_model()
