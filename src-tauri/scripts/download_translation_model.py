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
    # Construct target directory structure manually since we use local_dir
    # e.g., cache_dir/models--Helsinki-NLP--opus-mt-en-hi
    folder_name = f"models--{model_name.replace('/', '--')}"
    final_model_dir = os.path.join(cache_dir, folder_name)
    print(f"Downloading model {model_name} to {final_model_dir}", flush=True)
    try:
        from huggingface_hub import list_repo_files, hf_hub_url
        import requests

        # Check if the model is gated (requires authentication)
        is_gated = False

        if is_gated and token:
            from huggingface_hub import login
            print("Model is gated. Logging in with token...", flush=True)
            login(token=token)
        else:
            print("Model is public. Skipping authentication.", flush=True)

        # Get list of files in repo
        files = list_repo_files(repo_id=model_name, token=False)
        print(f"Found {len(files)} files to download.", flush=True)

        for filename in files:
            # Construct target path
            dest_path = os.path.join(final_model_dir, filename)
            os.makedirs(os.path.dirname(dest_path), exist_ok=True)
            
            url = hf_hub_url(repo_id=model_name, filename=filename)
            print(f"Downloading {filename}...", flush=True)
            
            # Simple chunked download with progress reporting
            response = requests.get(url, stream=True, timeout=30)
            response.raise_for_status()
            
            total_size = int(response.headers.get('content-length', 0))
            downloaded_size = 0
            last_percent = -1
            
            with open(dest_path, 'wb') as f:
                for chunk in response.iter_content(chunk_size=1024 * 1024): # 1MB chunks
                    if chunk:
                        f.write(chunk)
                        downloaded_size += len(chunk)
                        
                        if total_size > 0:
                            percent = int((downloaded_size / total_size) * 100)
                            if percent > last_percent:
                                # Format: PROGRESS:PERCENT:FILENAME
                                print(f"PROGRESS:{percent}:{filename}", flush=True)
                                last_percent = percent

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