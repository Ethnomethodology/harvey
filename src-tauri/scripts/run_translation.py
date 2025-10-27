# src-tauri/scripts/run_translation.py
import argparse
import sys
import json
from transformers import MarianMTModel, MarianTokenizer

def translate_line(text, model, tokenizer):
    """Translates a single line of text."""
    if not text.strip():
        return ""
    inputs = tokenizer(text, return_tensors="pt", padding=True, truncation=True, max_length=512)
    translated_tokens = model.generate(**inputs)
    return tokenizer.decode(translated_tokens[0], skip_special_tokens=True)

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Translate text using a local MarianMT model.")
    parser.add_argument("--model-path", required=True, help="Path to the local MarianMT model directory.")
    parser.add_argument("--text", required=True, help="A JSON string of a list of text segments to translate.")

    args = parser.parse_args()

    try:
        # Reconfigure stdout to ensure UTF-8 encoding, especially for Windows
        if sys.platform == "win32":
            sys.stdout.reconfigure(encoding='utf-8')
            
        sys.stderr.write(f"[Python Debug] Loading model and tokenizer from: {args.model_path}\n")
        tokenizer = MarianTokenizer.from_pretrained(args.model_path)
        model = MarianMTModel.from_pretrained(args.model_path)
        sys.stderr.write("[Python Debug] Model and tokenizer loaded.\n")

        segments = json.loads(args.text)
        translated_segments = [translate_line(segment, model, tokenizer) for segment in segments]

        final_output = json.dumps(translated_segments, ensure_ascii=False)
        sys.stderr.write(f"[Python Debug] Final translated JSON output:\n{final_output}\n")

        print(final_output)

    except Exception as e:
        print(f"Error during translation: {e}", file=sys.stderr)
        sys.exit(1)
