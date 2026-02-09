import sys
import os
import logging
from huggingface_hub import snapshot_download, login

# Configure logging
logging.basicConfig(level=logging.INFO, stream=sys.stdout)
logger = logging.getLogger('huggingface_hub')
logger.setLevel(logging.INFO)

def download_model(model_name, cache_dir, token):
    """
    Downloads a translation model from Hugging Face without loading it into RAM.
    Supports Helsinki-NLP and NLLB models.
    """
    print(f"Downloading model {model_name} to {cache_dir}", flush=True)
    try:
        if token:
            login(token=token)

        # Download the full repository snapshot
        # local_dir_use_symlinks=False ensures the files are actually moved into the dir
        # rather than just being symlinked from the HF cache.
        snapshot_download(
            repo_id=model_name,
            cache_dir=cache_dir,
            local_dir_use_symlinks=False,
            resume_download=True
        )

        print("Download complete.", flush=True)
    except Exception as e:
        print(f"Error downloading model: {e}", file=sys.stderr, flush=True)
        sys.exit(1)

if __name__ == "__main__":
    if len(sys.argv) != 4:
        print("Usage: python download_translation_model.py <model_name> <cache_dir> <token>", file=sys.stderr, flush=True)
        sys.exit(1)

    model_name = sys.argv[1]
    cache_dir = sys.argv[2]
    token = sys.argv[3]
    download_model(model_name, cache_dir, token)