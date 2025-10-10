import sys
import os
import logging
from transformers import MarianMTModel, MarianTokenizer
from huggingface_hub import login

# Configure logging for huggingface_hub
logging.basicConfig(level=logging.INFO, stream=sys.stdout)
logging.getLogger('huggingface_hub').setLevel(logging.INFO)

def download_model(model_name, cache_dir, token):
    """
    Downloads a MarianMT model and tokenizer to a specified cache directory.
    """
    print(f"Downloading model {model_name} to {cache_dir}", flush=True)
    try:
        # Ensure the cache directory exists
        os.makedirs(cache_dir, exist_ok=True)

        if token:
            login(token=token)

        # Download tokenizer and model
        tokenizer = MarianTokenizer.from_pretrained(model_name, cache_dir=cache_dir)
        model = MarianMTModel.from_pretrained(model_name, cache_dir=cache_dir)

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