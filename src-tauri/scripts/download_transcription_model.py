import sys
import os
import logging
from huggingface_hub import snapshot_download, login

# Configure logging
logging.basicConfig(level=logging.INFO, stream=sys.stdout)
logger = logging.getLogger('huggingface_hub')
logger.setLevel(logging.INFO)

def download_model(model_name, target_dir, token):
    """
    Downloads a transcription model (faster-whisper) from Hugging Face.
    """
    # Construct target directory based on model name to flatten structure
    # e.g., target_dir/models--Systran--faster-whisper-tiny
    # Actually, if we use local_dir, we can control exact path.
    # But to match existing pattern of `delete_model` which expects `models--...`,
    # we should construct that path here or in Rust.
    # Rust passes `target_dir` which is `.../transcription/faster-whisper`.
    # We should append the folder name here or Rust should pass the full final path.

    # Let's match the standard naming convention for consistency but flatten it.
    folder_name = f"models--{model_name.replace('/', '--')}"
    final_model_dir = os.path.join(target_dir, folder_name)

    print(f"Downloading model {model_name} to {final_model_dir}", flush=True)
    try:
        # Check if the model is gated (requires authentication)
        # Faster-Whisper models (Systran) are generally public.
        is_gated = False

        if is_gated and token and token.strip():
            print(f"Logging in with token...", flush=True)
            login(token=token)
        else:
            print("Model is public. Skipping authentication.", flush=True)

        # Download the full repository snapshot to a specific directory (flattened)
        snapshot_download(
            repo_id=model_name,
            local_dir=final_model_dir,
            local_dir_use_symlinks=False,
            resume_download=True,
            token=token if is_gated else None
        )

        print("Download complete.", flush=True)
    except ImportError as e:
        print(f"Error: Missing required library: {e}. Please ensure 'huggingface_hub' is installed.", file=sys.stderr, flush=True)
        sys.exit(1)
    except Exception as e:
        # Print full stack trace for better debugging
        import traceback
        traceback.print_exc(file=sys.stderr)
        print(f"Error downloading model: {e}", file=sys.stderr, flush=True)
        sys.exit(1)

if __name__ == "__main__":
    if len(sys.argv) != 4:
        print("Usage: python download_transcription_model.py <model_name> <cache_dir> <token>", file=sys.stderr, flush=True)
        sys.exit(1)

    model_name = sys.argv[1]
    cache_dir = sys.argv[2]
    token = sys.argv[3]
    download_model(model_name, cache_dir, token)
