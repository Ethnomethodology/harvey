import sys
import json
import logging
import urllib.request
import urllib.error

# Configure logging
logging.basicConfig(level=logging.INFO, stream=sys.stderr)
logger = logging.getLogger(__name__)

def fetch_json(url):
    """
    Helper to fetch JSON from a URL using only standard libraries.
    """
    try:
        req = urllib.request.Request(url, headers={'User-Agent': 'Harvey-Transcription-App/0.1.0'})
        with urllib.request.urlopen(req, timeout=15) as response:
            if response.status == 200:
                return json.loads(response.read().decode('utf-8'))
            else:
                logger.error(f"HTTP Error {response.status} for URL: {url}")
                return None
    except Exception as e:
        logger.error(f"Failed to fetch {url}: {e}")
        return None

def fetch_models():
    """
    Fetches Helsinki-NLP and NLLB models from Hugging Face and outputs a list of model objects.
    Uses standard libraries only to ensure compatibility with system Python.
    """
    model_list = []

    # 1. Fetch Helsinki-NLP models
    # API: https://huggingface.co/api/models?author=Helsinki-NLP&limit=3000
    helsinki_url = "https://huggingface.co/api/models?author=Helsinki-NLP&limit=3000"
    logger.info("Fetching Helsinki-NLP models from HF API...")
    helsinki_data = fetch_json(helsinki_url)
    
    if helsinki_data and isinstance(helsinki_data, list):
        for model in helsinki_data:
            model_id = model.get("modelId") or model.get("id")
            if not model_id or "opus-mt-" not in model_id:
                continue
            
            parts = model_id.split("/")
            if len(parts) != 2:
                continue
            
            name_parts = parts[1].split("-")
            src = None
            tgt = None
            
            if len(name_parts) >= 4 and name_parts[0] == "opus" and name_parts[1] == "mt":
                if name_parts[2] == "tc" and name_parts[3] == "big" and len(name_parts) >= 6:
                     src = name_parts[4]
                     tgt = name_parts[5]
                elif len(name_parts) == 4:
                     src = name_parts[2]
                     tgt = name_parts[3]

            model_list.append({
                "id": model_id,
                "downloads": model.get("downloads", 0),
                "likes": model.get("likes", 0),
                "last_modified": str(model.get("lastModified", "")),
                "src": src,
                "tgt": tgt,
                "family": "helsinki"
            })
    else:
        logger.warning("Could not retrieve Helsinki-NLP models or received invalid format.")

    # 2. Fetch NLLB models
    nllb_models = [
        "facebook/nllb-200-distilled-600M",
        "facebook/nllb-200-distilled-1.3B",
        "facebook/nllb-200-1.3B",
        "facebook/nllb-200-3.3B"
    ]
    
    logger.info("Fetching metadata for selected NLLB models...")
    for m_id in nllb_models:
        # API: https://huggingface.co/api/models/{m_id}
        m_url = f"https://huggingface.co/api/models/{m_id}"
        m_info = fetch_json(m_url)
        
        if m_info:
            model_list.append({
                "id": m_info.get("modelId") or m_info.get("id") or m_id,
                "downloads": m_info.get("downloads", 0),
                "likes": m_info.get("likes", 0),
                "last_modified": str(m_info.get("lastModified", "")),
                "src": "multi",
                "tgt": "multi",
                "family": "nllb"
            })
        else:
            # Fallback without stats if API fails for specific model
            model_list.append({
                "id": m_id,
                "downloads": 0,
                "likes": 0,
                "last_modified": "",
                "src": "multi",
                "tgt": "multi",
                "family": "nllb"
            })

    # Final sanity check: if the list is still empty, maybe there's a connectivity issue.
    # We output whatever we have.
    print(json.dumps(model_list))

if __name__ == "__main__":
    try:
        fetch_models()
    except Exception as e:
        logger.critical(f"Unhandled exception in fetch_models: {e}")
        print("[]")
        sys.exit(1)
