# src-tauri/scripts/download_diarization_model.py
import sys
import logging
from pyannote.audio import Pipeline

# Configure logging to capture detailed output from huggingface_hub
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(levelname)s - [%(name)s] - %(message)s',
    stream=sys.stdout, # Ensure logs go to stdout
)

def download_model(token):
    """
    Instantiates the pyannote.audio pipeline to trigger model download.
    """
    try:
        print("Attempting to download/load speaker diarization pipeline...", flush=True)
        pipeline = Pipeline.from_pretrained(
            "pyannote/speaker-diarization-3.1",
            token=token
        )
        print("Successfully loaded pipeline. Model is cached.", flush=True)
    except Exception as e:
        print(f"Error: Failed to download or load pipeline: {e}", file=sys.stderr, flush=True)
        sys.exit(1)

if __name__ == "__main__":
    if len(sys.argv) != 2:
        print("Usage: python download_diarization_model.py <HUGGING_FACE_TOKEN>", file=sys.stderr, flush=True)
        sys.exit(1)

    auth_token = sys.argv[1]
    download_model(auth_token)