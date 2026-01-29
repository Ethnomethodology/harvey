import sys
import json
import logging
from huggingface_hub import HfApi

# Configure logging
logging.basicConfig(level=logging.INFO, stream=sys.stderr)
logger = logging.getLogger(__name__)

def fetch_models():
    """
    Fetches Helsinki-NLP models from Hugging Face and outputs a list of model objects.
    """
    try:
        api = HfApi()
        # Fetch models - removed full=True to avoid timeouts
        models = api.list_models(author="Helsinki-NLP", limit=3000) 

        model_list = []

        for model in models:
            model_id = model.modelId
            
            # Filter for opus-mt models
            if "opus-mt-" not in model_id:
                continue
            
            # Skip TC models for now if desired, or include them. 
            # The user asked for "more info", so let's include them but parse them correctly if we can.
            # Standard: Helsinki-NLP/opus-mt-{src}-{tgt}
            
            parts = model_id.split("/")
            if len(parts) != 2:
                continue
            
            name_parts = parts[1].split("-")
            
            # Try to extract src/tgt
            # Case 1: opus-mt-en-fr (4 parts)
            # Case 2: opus-mt-tc-big-en-fr (6 parts)
            
            src = None
            tgt = None
            
            if len(name_parts) >= 4 and name_parts[0] == "opus" and name_parts[1] == "mt":
                if name_parts[2] == "tc" and name_parts[3] == "big" and len(name_parts) >= 6:
                     # opus-mt-tc-big-en-fr
                     src = name_parts[4]
                     tgt = name_parts[5]
                elif len(name_parts) == 4:
                     # opus-mt-en-fr
                     src = name_parts[2]
                     tgt = name_parts[3]
                else:
                    # fallback for complex codes like 'opus-mt-es-en_el_es' or similar if they exist
                    # just take the last two? No, unsafe.
                    pass

            # Prepare the object
            model_data = {
                "id": model_id,
                "downloads": getattr(model, "downloads", 0),
                "likes": getattr(model, "likes", 0),
                "last_modified": str(getattr(model, "lastModified", "")),
                "src": src,
                "tgt": tgt
            }
            model_list.append(model_data)

        # Output JSON list to stdout
        print(json.dumps(model_list))
        
    except Exception as e:
        logger.error(f"Error fetching models: {e}")
        # Return empty list on error so frontend doesn't crash
        print("[]")

if __name__ == "__main__":
    fetch_models()