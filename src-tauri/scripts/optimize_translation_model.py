import sys
import os
import shutil
import ctranslate2
import transformers

def optimize_model(model_path, output_path):
    """
    Converts a Transformers model to CTranslate2 format.
    Uses INT8 quantization for CPU efficiency.
    """
    print(f"Optimizing model at {model_path} -> {output_path}", flush=True)
    
    try:
        # Check if it's already optimized
        if os.path.exists(output_path):
            print(f"Output directory {output_path} already exists. Removing...", flush=True)
            shutil.rmtree(output_path)

        converter = ctranslate2.converters.TransformersConverter(model_path)
        
        # INT8 is generally safe and much faster on CPU
        converter.convert(
            output_path,
            quantization="int8",
            force=True
        )
        
        print("Optimization complete.", flush=True)
    except Exception as e:
        print(f"Error during optimization: {e}", file=sys.stderr, flush=True)
        sys.exit(1)

if __name__ == "__main__":
    if len(sys.argv) != 3:
        print("Usage: python optimize_translation_model.py <model_path> <output_path>", file=sys.stderr, flush=True)
        sys.exit(1)

    model_path = sys.argv[1]
    output_path = sys.argv[2]
    optimize_model(model_path, output_path)
