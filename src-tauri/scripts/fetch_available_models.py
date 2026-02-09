import sys
import json
import logging
from huggingface_hub import HfApi

# Configure logging
logging.basicConfig(level=logging.INFO, stream=sys.stderr)
logger = logging.getLogger(__name__)

def fetch_models():
    """
    Fetches Helsinki-NLP and NLLB models from Hugging Face and outputs a list of model objects.
    """
    try:
        api = HfApi()
        
        model_list = []

        # 1. Fetch Helsinki-NLP models
        models = api.list_models(author="Helsinki-NLP", limit=3000) 
        for model in models:
            model_id = model.modelId
            if "opus-mt-" not in model_id:
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
                "downloads": getattr(model, "downloads", 0),
                "likes": getattr(model, "likes", 0),
                "last_modified": str(getattr(model, "lastModified", "")),
                "src": src,
                "tgt": tgt,
                "family": "helsinki"
            })

        # 2. Fetch NLLB models
        # We manually specify some popular NLLB models as listing them might be messy
        nllb_models = [
            "facebook/nllb-200-distilled-600M",
            "facebook/nllb-200-distilled-1.3B",
            "facebook/nllb-200-1.3B",
            "facebook/nllb-200-3.3B"
        ]
        
        # We fetch their metadata to get download/like stats if possible
        for m_id in nllb_models:
            try:
                m_info = api.model_info(m_id)
                model_list.append({
                    "id": m_info.modelId,
                    "downloads": getattr(m_info, "downloads", 0),
                    "likes": getattr(m_info, "likes", 0),
                    "last_modified": str(getattr(m_info, "lastModified", "")),
                    "src": "multi",
                    "tgt": "multi",
                    "family": "nllb"
                })
            except Exception as e:
                logger.warn(f"Could not fetch info for {m_id}: {e}")
                # Fallback without stats
                model_list.append({
                    "id": m_id,
                    "downloads": 0,
                    "likes": 0,
                    "last_modified": "",
                    "src": "multi",
                    "tgt": "multi",
                    "family": "nllb"
                })

        # Output JSON list to stdout
        print(json.dumps(model_list))
        
    except Exception as e:
        logger.error(f"Error fetching models: {e}")
        # Return empty list on error so frontend doesn't crash
        print("[]")

if __name__ == "__main__":
    fetch_models()