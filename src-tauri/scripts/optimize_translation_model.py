import sys
import os
import shutil
import ctranslate2
import transformers

def optimize_model(model_path, output_path, quantization=None):
    """
    Converts a Transformers model to CTranslate2 format.
    Uses optimal quantization based on model family unless overridden.
    """
    print(f"Optimizing model at {model_path} -> {output_path}", flush=True)
    
    try:
        # Check if it's already optimized
        if os.path.exists(output_path):
            print(f"Output directory {output_path} already exists. Removing...", flush=True)
            shutil.rmtree(output_path)

        # Detect family
        is_nllb = "nllb" in model_path.lower()
        
        print("Loading source model into converter (this may take a moment)...", flush=True)
        converter = ctranslate2.converters.TransformersConverter(model_path)
        
        # Determine quantization: 
        if quantization and quantization in ["int8", "float16", "int8_float16", "int16"]:
            quant = quantization
            print(f"Using explicitly requested quantization: {quant}", flush=True)
        else:
            # NLLB models are large and benefit greatly from int8 on CPU.
            # Helsinki models also benefit from int8 for speed on CPU, which is the priority.
            quant = "int8"
            print(f"Using default quantization for {'NLLB' if is_nllb else 'Helsinki'}: {quant}", flush=True)
        
        print(f"Starting conversion to CTranslate2 format with {quant} quantization...", flush=True)
        converter.convert(
            output_path,
            quantization=quant,
            force=True
        )
        
        print("Optimization complete.", flush=True)
    except Exception as e:
        print(f"Error during optimization: {e}", file=sys.stderr, flush=True)
        sys.exit(1)

if __name__ == "__main__":
    if len(sys.argv) < 3:
        print("Usage: python optimize_translation_model.py <model_path> <output_path> [quantization]", file=sys.stderr, flush=True)
        sys.exit(1)

    model_path = sys.argv[1]
    output_path = sys.argv[2]
    quant_arg = sys.argv[3] if len(sys.argv) > 3 else None

    optimize_model(model_path, output_path, quantization=quant_arg)
