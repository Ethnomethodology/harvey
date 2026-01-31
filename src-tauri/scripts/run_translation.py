# src-tauri/scripts/run_translation.py
import argparse
import sys
import os
import json
import torch
import re
from transformers import AutoModelForSeq2SeqLM, AutoTokenizer, MarianMTModel, MarianTokenizer

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

def translate_sliding_window(segments, engine, tokenizer, device, batch_size=8, src_lang=None, tgt_lang=None):
    """
    Translates segments using a sliding window approach: [Prev] + [Target] + [Next].
    This provides context to the model.
    """
    final_translations = [None] * len(segments)
    
    use_ct2 = hasattr(engine, "translate_batch")
    is_nllb = not use_ct2 and hasattr(engine, "config") and "nllb" in engine.config._name_or_path.lower()

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
            
            # Construct input with newlines as strong separators
            # MarianMT models typically respect newlines as sentence boundaries
            parts = []
            if prev_txt: parts.append(prev_txt)
            parts.append(text)
            if next_txt: parts.append(next_txt)
            
            batch_inputs.append("\n".join(parts))
            batch_metadata.append((bool(prev_txt), bool(next_txt)))
            
        valid_indices_map = [k for k, t in enumerate(batch_inputs) if t]
        valid_inputs = [batch_inputs[k] for k in valid_indices_map]
        
        if valid_inputs:
            if use_ct2:
                # CTranslate2: Use proper tokenization with special tokens
                source_tokens = [tokenizer.convert_ids_to_tokens(tokenizer.encode(t, add_special_tokens=True)) for t in valid_inputs]
                results = engine.translate_batch(source_tokens, asynchronous=False)
                decoded = [tokenizer.decode(tokenizer.convert_tokens_to_ids(r.hypotheses[0]), skip_special_tokens=True) for r in results]
            else:
                # Transformers: Robust multi-line generation
                inputs = tokenizer(valid_inputs, return_tensors="pt", padding=True, truncation=True).to(device)
                with torch.no_grad():
                    if is_nllb and tgt_lang:
                        # NLLB needs forced_bos_token_id for target language
                        tgt_lang_id = tokenizer.convert_tokens_to_ids(tgt_lang)
                        generated = engine.generate(**inputs, forced_bos_token_id=tgt_lang_id)
                    else:
                        generated = engine.generate(**inputs)
                decoded = tokenizer.batch_decode(generated, skip_special_tokens=True)
            
            for map_idx, output_text in zip(valid_indices_map, decoded):
                has_prev, has_next = batch_metadata[map_idx]
                expected_count = 1 + int(has_prev) + int(has_next)
                
                # Split output by newline and filter empty
                lines = [line.strip() for line in output_text.split('\n') if line.strip()]
                
                if len(lines) == expected_count:
                    # Perfect structure: Extract the target segment
                    target_line_idx = 1 if has_prev else 0
                    final_translations[batch_indices[map_idx]] = lines[target_line_idx]
                else:
                    # Logic failure or model merged lines. 
                    # Leave as None to trigger isolation fallback below.
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
            
            if use_ct2:
                # CTranslate2: Process individually for robust mapping
                decoded = []
                for t in batch_texts:
                    source_tokens = tokenizer.convert_ids_to_tokens(tokenizer.encode(t, add_special_tokens=True))
                    results = engine.translate_batch([source_tokens], asynchronous=False)
                    output_text = tokenizer.decode(tokenizer.convert_tokens_to_ids(results[0].hypotheses[0]), skip_special_tokens=True)
                    decoded.append(output_text)
            else:
                inputs = tokenizer(batch_texts, return_tensors="pt", padding=True, truncation=True).to(device)
                with torch.no_grad():
                    if is_nllb and tgt_lang:
                        tgt_lang_id = tokenizer.convert_tokens_to_ids(tgt_lang)
                        generated = engine.generate(**inputs, forced_bos_token_id=tgt_lang_id)
                    else:
                        generated = engine.generate(**inputs)
                decoded = tokenizer.batch_decode(generated, skip_special_tokens=True)
            
            for ix, trans in zip(batch_ixs, decoded):
                final_translations[ix] = trans.strip()
                
    return final_translations

def translate_bulk(segments, engine, tokenizer, device, batch_size=16, src_lang=None, tgt_lang=None):
    """
    Standard batch translation.
    """
    use_ct2 = hasattr(engine, "translate_batch")
    is_nllb = not use_ct2 and hasattr(engine, "config") and "nllb" in engine.config._name_or_path.lower()
    translated = []
    for i in range(0, len(segments), batch_size):
        batch = segments[i : i + batch_size]
        
        if use_ct2:
            # CTranslate2: Process individually for highest quality mapping
            for text in batch:
                if not text.strip():
                    translated.append("")
                    continue
                source_tokens = tokenizer.convert_ids_to_tokens(tokenizer.encode(text, add_special_tokens=True))
                results = engine.translate_batch([source_tokens], asynchronous=False)
                output_text = tokenizer.decode(tokenizer.convert_tokens_to_ids(results[0].hypotheses[0]), skip_special_tokens=True)
                translated.append(output_text.strip())
        else:
            batch_clean = [s if s.strip() else " " for s in batch] 
            inputs = tokenizer(batch_clean, return_tensors="pt", padding=True, truncation=True).to(device)
            with torch.no_grad():
                if is_nllb and tgt_lang:
                    tgt_lang_id = tokenizer.convert_tokens_to_ids(tgt_lang)
                    generated = engine.generate(**inputs, forced_bos_token_id=tgt_lang_id)
                else:
                    generated = engine.generate(**inputs)
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
        
        # Heuristic to check for NLLB
        is_nllb = "nllb" in args.model_path.lower()
        
        # Determine language codes for NLLB
        nllb_src = get_nllb_lang_code(args.src_lang) if is_nllb else None
        nllb_tgt = get_nllb_lang_code(args.tgt_lang) if is_nllb else None

        sys.stderr.write(f"[Python Debug] Loading model from {args.model_path} (is_nllb={is_nllb})...\n")
        
        # Check for CTranslate2 optimized model
        ct2_model_path = os.path.join(args.model_path, "ct2_optimized")
        use_ct2 = not is_nllb and os.path.exists(ct2_model_path)
        
        engine = None
        if use_ct2:
            try:
                import ctranslate2
                sys.stderr.write(f"[Python Debug] Using CTranslate2 optimized engine: {ct2_model_path}\n")
                engine = ctranslate2.Translator(ct2_model_path, device="cpu")
                # For Helsinki models, MarianTokenizer is still preferred even with CT2
                tokenizer = MarianTokenizer.from_pretrained(args.model_path)
            except ImportError:
                sys.stderr.write("[Python Debug] CTranslate2 not found, falling back to transformers.\n")
                use_ct2 = False

        if not use_ct2:
            sys.stderr.write("[Python Debug] Loading with transformers engine.\n")
            if is_nllb:
                tokenizer = AutoTokenizer.from_pretrained(args.model_path, src_lang=nllb_src)
                engine = AutoModelForSeq2SeqLM.from_pretrained(args.model_path).to(device)
            else:
                tokenizer = MarianTokenizer.from_pretrained(args.model_path)
                engine = MarianMTModel.from_pretrained(args.model_path).to(device)

        segments = json.loads(args.text)
        
        results = []
        if args.mode == "document":
            sys.stderr.write(f"[Python Debug] Translating {len(segments)} segments in Document mode (bulk)...")
            results = translate_bulk(segments, engine, tokenizer, device, src_lang=nllb_src, tgt_lang=nllb_tgt)
        else:
            sys.stderr.write(f"[Python Debug] Translating {len(segments)} segments in Transcript mode (sliding window)...")
            results = translate_sliding_window(segments, engine, tokenizer, device, src_lang=nllb_src, tgt_lang=nllb_tgt)
            
        print(json.dumps(results, ensure_ascii=False))

    except Exception as e:
        sys.stderr.write(f"Translation Error: {e}\n")
        import traceback
        traceback.print_exc(file=sys.stderr)
        sys.exit(1)