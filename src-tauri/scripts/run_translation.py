# src-tauri/scripts/run_translation.py
import argparse
import sys
import os
import json
import torch
import re
from transformers import AutoModelForSeq2SeqLM, AutoTokenizer

# PyTorch 2.6+ fix
try:
    from torch.torch_version import TorchVersion
    if hasattr(torch, "serialization") and hasattr(torch.serialization, "add_safe_globals"):
        torch.serialization.add_safe_globals([TorchVersion])
except Exception:
    pass

# Mapping for NLLB language codes (2-letter to NLLB specific)
NLLB_LANG_MAP = {
    "en": "eng_Latn",
    "ja": "jpn_Jpan",
    "fr": "fra_Latn",
    "de": "deu_Latn",
    "es": "spa_Latn",
    "it": "ita_Latn",
    "ko": "kor_Hang",
    "zh": "zho_Hans",
    "pt": "por_Latn",
    "ru": "rus_Cyrl",
    "nl": "nld_Latn",
    "ar": "ara_Arab",
    "tr": "tur_Latn",
    "hi": "hin_Deva",
    "vi": "vie_Latn",
    "th": "tha_Thai",
}

def get_nllb_lang_code(lang_code):
    if not lang_code:
        return None
    # If already in NLLB format, return as is
    if "_" in lang_code and len(lang_code) == 8:
        return lang_code
    return NLLB_LANG_MAP.get(lang_code.lower())

def translate_sliding_window(segments, model, tokenizer, device, batch_size=8, src_lang=None, tgt_lang=None):
    """
    Translates segments using a sliding window approach: [Prev] + [Target] + [Next].
    """
    final_translations = [None] * len(segments)
    
    is_nllb = "nllb" in model.config._name_or_path.lower()

    # 1. Contextual Pass
    for i in range(0, len(segments), batch_size):
        batch_indices = range(i, min(i + batch_size, len(segments)))
        batch_inputs = []
        batch_metadata = [] 
        
        for idx in batch_indices:
            text = segments[idx].strip()
            if not text:
                batch_inputs.append("")
                batch_metadata.append((False, False))
                continue

            prev_txt = segments[idx-1].strip() if idx > 0 else ""
            next_txt = segments[idx+1].strip() if idx < len(segments) - 1 else ""
            
            parts = []
            if prev_txt: parts.append(prev_txt)
            parts.append(text)
            if next_txt: parts.append(next_txt)
            
            batch_inputs.append("\n".join(parts))
            batch_metadata.append((bool(prev_txt), bool(next_txt)))
            
        valid_indices_map = [k for k, t in enumerate(batch_inputs) if t]
        valid_inputs = [batch_inputs[k] for k in valid_indices_map]
        
        if valid_inputs:
            # Tokenize
            inputs = tokenizer(valid_inputs, return_tensors="pt", padding=True, truncation=True).to(device)
            
            # Generate
            with torch.no_grad():
                if is_nllb and tgt_lang:
                    # NLLB needs forced_bos_token_id for target language
                    tgt_lang_id = tokenizer.convert_tokens_to_ids(tgt_lang)
                    generated = model.generate(**inputs, forced_bos_token_id=tgt_lang_id)
                else:
                    generated = model.generate(**inputs)
            
            decoded = tokenizer.batch_decode(generated, skip_special_tokens=True)
            
            for map_idx, output_text in zip(valid_indices_map, decoded):
                has_prev, has_next = batch_metadata[map_idx]
                expected_count = 1 + int(has_prev) + int(has_next)
                
                lines = [line.strip() for line in output_text.split('\n') if line.strip()]
                
                if len(lines) == expected_count:
                    target_line_idx = 1 if has_prev else 0
                    final_translations[batch_indices[map_idx]] = lines[target_line_idx]
                else:
                    pass
        
        for k, text in enumerate(batch_inputs):
            if not text:
                final_translations[batch_indices[k]] = ""

    # 2. Fallback Pass: Isolation
    failed_indices = [ix for ix, r in enumerate(final_translations) if r is None]
    
    if failed_indices:
        sys.stderr.write(f"[Info] {len(failed_indices)} segments failed context check. Falling back to isolation.\n")
        
        for i in range(0, len(failed_indices), batch_size):
            batch_ixs = failed_indices[i : i + batch_size]
            batch_texts = [segments[ix] for ix in batch_ixs]
            
            inputs = tokenizer(batch_texts, return_tensors="pt", padding=True, truncation=True).to(device)
            with torch.no_grad():
                if is_nllb and tgt_lang:
                    tgt_lang_id = tokenizer.convert_tokens_to_ids(tgt_lang)
                    generated = model.generate(**inputs, forced_bos_token_id=tgt_lang_id)
                else:
                    generated = model.generate(**inputs)
            
            decoded = tokenizer.batch_decode(generated, skip_special_tokens=True)
            
            for ix, trans in zip(batch_ixs, decoded):
                final_translations[ix] = trans.strip()
                
    return final_translations

def translate_bulk(segments, model, tokenizer, device, batch_size=16, src_lang=None, tgt_lang=None):
    """
    Standard batch translation.
    """
    is_nllb = "nllb" in model.config._name_or_path.lower()
    translated = []
    for i in range(0, len(segments), batch_size):
        batch = segments[i : i + batch_size]
        batch_clean = [s if s.strip() else " " for s in batch] 
        
        inputs = tokenizer(batch_clean, return_tensors="pt", padding=True, truncation=True).to(device)
        with torch.no_grad():
            if is_nllb and tgt_lang:
                tgt_lang_id = tokenizer.convert_tokens_to_ids(tgt_lang)
                generated = model.generate(**inputs, forced_bos_token_id=tgt_lang_id)
            else:
                generated = model.generate(**inputs)
        
        decoded = tokenizer.batch_decode(generated, skip_special_tokens=True)
        
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
    parser.add_argument("--src-lang", help="Source language code (e.g. en, eng_Latn)")
    parser.add_argument("--tgt-lang", help="Target language code (e.g. ja, jpn_Jpan)")
    args = parser.parse_args()

    try:
        os.environ["HF_HUB_OFFLINE"] = "1"
        if sys.platform == "win32":
            sys.stdout.reconfigure(encoding='utf-8')
            
        device = "cuda" if torch.cuda.is_available() else "cpu"
        
        is_nllb = "nllb" in args.model_path.lower()
        
        # Determine language codes for NLLB
        nllb_src = get_nllb_lang_code(args.src_lang) if is_nllb else None
        nllb_tgt = get_nllb_lang_code(args.tgt_lang) if is_nllb else None

        sys.stderr.write(f"[Python Debug] Loading model from {args.model_path} (is_nllb={is_nllb})...\n")
        
        if is_nllb:
            tokenizer = AutoTokenizer.from_pretrained(args.model_path, src_lang=nllb_src)
        else:
            tokenizer = AutoTokenizer.from_pretrained(args.model_path)
            
        model = AutoModelForSeq2SeqLM.from_pretrained(args.model_path).to(device)

        segments = json.loads(args.text)
        
        results = []
        if args.mode == "document":
            sys.stderr.write(f"[Python Debug] Translating {len(segments)} segments in Document mode (bulk)...\n")
            results = translate_bulk(segments, model, tokenizer, device, src_lang=nllb_src, tgt_lang=nllb_tgt)
        else:
            sys.stderr.write(f"[Python Debug] Translating {len(segments)} segments in Transcript mode (sliding window)...\n")
            results = translate_sliding_window(segments, model, tokenizer, device, src_lang=nllb_src, tgt_lang=nllb_tgt)
            
        print(json.dumps(results, ensure_ascii=False))

    except Exception as e:
        sys.stderr.write(f"Translation Error: {e}\n")
        import traceback
        traceback.print_exc(file=sys.stderr)
        sys.exit(1)
