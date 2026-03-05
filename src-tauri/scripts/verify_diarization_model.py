# src-tauri/scripts/verify_diarization_model.py
import os
import sys
import torch
from pyannote.audio import Pipeline

def verify_model():
    try:
        # Force offline mode to ensure we're loading from local cache
        os.environ["HF_HUB_OFFLINE"] = "1"
        
        # Load the pipeline
        pipeline = Pipeline.from_pretrained("pyannote/speaker-diarization-3.1")
        
        if pipeline is not None:
            print("verified")
        else:
            print("failed: Pipeline is None")
    except Exception as e:
        print(f"failed: {str(e)}")

if __name__ == "__main__":
    verify_model()
