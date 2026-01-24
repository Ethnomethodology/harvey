# src-tauri/scripts/run_translation.py
import argparse
import sys
import os
import json
import torch
import re
from transformers import MarianMTModel, MarianTokenizer

# PyTorch 2.6+ fix
try:
    from torch.torch_version import TorchVersion
    if hasattr(torch, "serialization") and hasattr(torch.serialization, "add_safe_globals"):
        torch.serialization.add_safe_globals([TorchVersion])
except Exception:
    pass

def split_into_sentences(text):
    """
    Splits a block of text into sentences using common punctuation.
    Used to recover segments if the model merges lines.
    """
    # Split by punctuation followed by space or newline
    sentences = re.split(r'(?<=[.!?])\s+', text.strip())
    return [s.strip() for s in sentences if s.strip()]

def distribute_translated_to_original(translated_lines, original_texts):
    """
    Intelligently maps a list of translated sentences/lines back to 
    the original segment count.
    """
    target_count = len(original_texts)
    
    # If we have a perfect match, return
    if len(translated_lines) == target_count:
        return translated_lines
        
    # If we have too many lines, join the extras into the last segment
    if len(translated_lines) > target_count:
        result = translated_lines[:target_count-1]
        result.append(" ".join(translated_lines[target_count-1:]))
        return result
        
    # If we have too few lines (merging happened), use character-ratio distribution
    # but first try to split the existing lines into more sentences
    all_sentences = []
    for line in translated_lines:
        all_sentences.extend(split_into_sentences(line))
        
    if len(all_sentences) == target_count:
        return all_sentences
        
    # Final fallback: Proportionally split the entire block of text
    full_text = " ".join(translated_lines)
    orig_lengths = [len(t) for t in original_texts]
    total_orig_len = sum(orig_lengths)
    if total_orig_len == 0: return [""] * target_count
    
    words = full_text.split()
    if not words: return [""] * target_count
    
    result = []
    current_word_idx = 0
    total_trans_chars = len(full_text)
    cum_orig_len = 0
    
    for i in range(target_count):
        if i == target_count - 1:
            result.append(" ".join(words[current_word_idx:]))
        else:
            cum_orig_len += orig_lengths[i]
            target_cum_pos = (cum_orig_len / total_orig_len) * total_trans_chars
            seg_words = []
            while current_word_idx < len(words):
                word = words[current_word_idx]
                test_str = " ".join(result + seg_words + [word])
                if not seg_words or len(test_str) <= target_cum_pos:
                    seg_words.append(word)
                    current_word_idx += 1
                else:
                    break
            result.append(" ".join(seg_words))
    return result

def translate_batch(texts, model, tokenizer, device):
    """
    Translates a batch of segments using newlines as natural context boundaries.
    """
    if not any(t.strip() for t in texts):
        return [""] * len(texts)

    # Join with newlines - this is the most 'natural' context for the model
    combined_input = "\n".join(texts)
    
    inputs = tokenizer(combined_input, return_tensors="pt", padding=True, truncation=True).to(device)
    with torch.no_grad():
        translated_tokens = model.generate(**inputs)
    translated_output = tokenizer.decode(translated_tokens[0], skip_special_tokens=True)
    
    # Split by the model's output lines
    translated_lines = [line.strip() for line in translated_output.split("\n") if line.strip()]
    
    # Map back to original segments
    return distribute_translated_to_original(translated_lines, texts)

if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--model-path", required=True)
    parser.add_argument("--text", required=True)
    args = parser.parse_args()

    try:
        os.environ["HF_HUB_OFFLINE"] = "1"
        if sys.platform == "win32":
            sys.stdout.reconfigure(encoding='utf-8')
            
        device = "cuda" if torch.cuda.is_available() else "cpu"
        tokenizer = MarianTokenizer.from_pretrained(args.model_path)
        model = MarianMTModel.from_pretrained(args.model_path).to(device)

        segments = json.loads(args.text)
        sys.stderr.write(f"[Python Debug] Translating {len(segments)} segments with linguistic context...\n")
        
        final_results = []
        batch_size = 5 # Good balance between context window and model stability
        
        for i in range(0, len(segments), batch_size):
            current_batch = segments[i : i + batch_size]
            final_results.extend(translate_batch(current_batch, model, tokenizer, device))

        # Ensure final count matches
        while len(final_results) < len(segments):
            final_results.append("")
            
        print(json.dumps(final_results[:len(segments)], ensure_ascii=False))

    except Exception as e:
        sys.stderr.write(f"Translation Error: {e}\n")
        sys.exit(1)
