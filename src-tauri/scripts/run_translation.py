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

def translate_sliding_window(segments, model, tokenizer, device, batch_size=8):
    """
    Translates segments using a sliding window approach: [Prev] + [Target] + [Next].
    This provides context to the model.
    If the model output preserves the structure (3 lines), we extract the middle line.
    If the model merges lines or hallucinates, we mark it as failed and use a 
    fallback isolation pass to guarantee 1:1 mapping.
    """
    final_translations = [None] * len(segments)
    
    # 1. Contextual Pass
    # We process in batches to maintain reasonable speed
    for i in range(0, len(segments), batch_size):
        batch_indices = range(i, min(i + batch_size, len(segments)))
        batch_inputs = []
        batch_metadata = [] # Stores (has_prev, has_next) used for extraction logic
        
        for idx in batch_indices:
            text = segments[idx].strip()
            if not text:
                # Empty segment, no need to translate
                batch_inputs.append("")
                batch_metadata.append((False, False))
                continue

            prev_txt = segments[idx-1].strip() if idx > 0 else ""
            next_txt = segments[idx+1].strip() if idx < len(segments) - 1 else ""
            
            # Construct input with newlines as strong separators
            # MarianMT models typically respect newlines as sentence boundaries
            parts = []
            if prev_txt: parts.append(prev_txt)
            parts.append(text)
            if next_txt: parts.append(next_txt)
            
            batch_inputs.append("\n".join(parts))
            batch_metadata.append((bool(prev_txt), bool(next_txt)))
            
        # Filter out empty inputs to skip inference
        valid_indices_map = [k for k, t in enumerate(batch_inputs) if t]
        valid_inputs = [batch_inputs[k] for k in valid_indices_map]
        
        if valid_inputs:
            inputs = tokenizer(valid_inputs, return_tensors="pt", padding=True, truncation=True).to(device)
            with torch.no_grad():
                generated = model.generate(**inputs)
            decoded = tokenizer.batch_decode(generated, skip_special_tokens=True)
            
            # Map results back to the original slots
            for map_idx, output_text in zip(valid_indices_map, decoded):
                has_prev, has_next = batch_metadata[map_idx]
                expected_count = 1 + int(has_prev) + int(has_next)
                
                # Split output by newline
                lines = [line.strip() for line in output_text.split('\n') if line.strip()]
                
                if len(lines) == expected_count:
                    # Perfect structure: Extract the target segment
                    # If we had a previous segment, the target is at index 1, otherwise 0
                    target_line_idx = 1 if has_prev else 0
                    final_translations[batch_indices[map_idx]] = lines[target_line_idx]
                else:
                    # Structure mismatch (model merged sentences or hallucinated breaks)
                    # Leave as None to trigger fallback isolation pass
                    pass
        
        # Handle the empty strings explicitly
        for k, text in enumerate(batch_inputs):
            if not text:
                final_translations[batch_indices[k]] = ""

    # 2. Fallback Pass: Isolation
    # Translate segments individually if context pass failed to return clean structure
    failed_indices = [ix for ix, r in enumerate(final_translations) if r is None]
    
    if failed_indices:
        sys.stderr.write(f"[Info] {len(failed_indices)} segments failed context check. Falling back to isolation.\n")
        
        for i in range(0, len(failed_indices), batch_size):
            batch_ixs = failed_indices[i : i + batch_size]
            batch_texts = [segments[ix] for ix in batch_ixs]
            
            inputs = tokenizer(batch_texts, return_tensors="pt", padding=True, truncation=True).to(device)
            with torch.no_grad():
                generated = model.generate(**inputs)
            decoded = tokenizer.batch_decode(generated, skip_special_tokens=True)
            
            for ix, trans in zip(batch_ixs, decoded):
                final_translations[ix] = trans.strip()
                
    return final_translations

def translate_bulk(segments, model, tokenizer, device, batch_size=16):
    """
    Standard batch translation. 
    Useful for documents where context is implicit in the flow, 
    but we don't need strict 1:1 structure verification for timestamps.
    """
    translated = []
    for i in range(0, len(segments), batch_size):
        batch = segments[i : i + batch_size]
        # Handle empty strings to avoid model errors or weird outputs
        batch_clean = [s if s.strip() else " " for s in batch] 
        
        inputs = tokenizer(batch_clean, return_tensors="pt", padding=True, truncation=True).to(device)
        with torch.no_grad():
            generated = model.generate(**inputs)
        decoded = tokenizer.batch_decode(generated, skip_special_tokens=True)
        
        # Restore empty strings
        for j, text in enumerate(batch):
            if not text.strip():
                decoded[j] = ""
        
        translated.extend(decoded)
    return translated

if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--model-path", required=True)
    parser.add_argument("--text", required=True)
    parser.add_argument("--mode", choices=["transcript", "document"], default="transcript")
    args = parser.parse_args()

    try:
        os.environ["HF_HUB_OFFLINE"] = "1"
        if sys.platform == "win32":
            sys.stdout.reconfigure(encoding='utf-8')
            
        device = "cuda" if torch.cuda.is_available() else "cpu"
        tokenizer = MarianTokenizer.from_pretrained(args.model_path)
        model = MarianMTModel.from_pretrained(args.model_path).to(device)

        segments = json.loads(args.text)
        
        results = []
        if args.mode == "document":
            sys.stderr.write(f"[Python Debug] Translating {len(segments)} segments in Document mode (bulk)...\n")
            results = translate_bulk(segments, model, tokenizer, device)
        else:
            sys.stderr.write(f"[Python Debug] Translating {len(segments)} segments in Transcript mode (sliding window)...\n")
            results = translate_sliding_window(segments, model, tokenizer, device)
            
        print(json.dumps(results, ensure_ascii=False))

    except Exception as e:
        sys.stderr.write(f"Translation Error: {e}\n")
        sys.exit(1)
