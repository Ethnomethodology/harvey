# src-tauri/scripts/run_translation.py
import argparse
import sys
from transformers import MarianMTModel, MarianTokenizer

def translate(text, model_path):
    """
    Translates a given text using a local MarianMT model.

    Args:
        text (str): The text to translate.
        model_path (str): The local file path to the pre-trained MarianMT model directory.
    """
    try:
        sys.stderr.write(f"[Python Debug] Starting translation with model: {model_path}\n")
        # Load the tokenizer and model from the specified local path
        tokenizer = MarianTokenizer.from_pretrained(model_path)
        model = MarianMTModel.from_pretrained(model_path)
        sys.stderr.write("[Python Debug] Model and tokenizer loaded.\n")

        # The transformers pipeline handles tokenization, generation, and decoding.
        # It's better to handle batching for longer texts if necessary.
        # MarianMT models have a max length, so we should process text in chunks if it's very long.
        # For now, we assume segments are passed one by one, which should be short enough.

        # Tokenize the input text
        inputs = tokenizer(text, return_tensors="pt", padding=True, truncation=True, max_length=512)

        # Generate the translation tokens
        translated_tokens = model.generate(**inputs)

        # Decode the tokens into a string
        translated_text = tokenizer.decode(translated_tokens[0], skip_special_tokens=True)
        sys.stderr.write(f"[Python Debug] Translated text: {translated_text}\n")

        # Print the final translated text to standard output
        print(translated_text)

    except Exception as e:
        # If any error occurs, print it to stderr and exit
        print(f"Error during translation with model {model_path}: {e}", file=sys.stderr)
        sys.exit(1)

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Translate text using a local MarianMT model.")
    parser.add_argument("--model-path", required=True, help="Path to the local MarianMT model directory.")

    args = parser.parse_args()

    # Read the entire text to be translated from standard input
    text_to_translate = sys.stdin.read()
    sys.stderr.write(f"[Python Debug] Received text to translate (first 100 chars): {text_to_translate[:100]}\n")

    if not text_to_translate.strip():
        # Handle cases where input is empty or just whitespace
        # The calling Rust code should ideally prevent this, but it's good practice.
        print("", end="")
        sys.exit(0)

    # Perform the translation
    translate(text_to_translate, args.model_path)
