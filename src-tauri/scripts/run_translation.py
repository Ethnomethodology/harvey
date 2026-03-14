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
    # Extended mappings
    "bn": "ben_Beng",
    "pl": "pol_Latn",
    "sv": "swe_Latn",
    "da": "dan_Latn",
    "fi": "fin_Latn",
    "no": "nob_Latn",
    "cs": "ces_Latn",
    "el": "ell_Grek",
    "he": "heb_Hebr",
    "id": "ind_Latn",
    "uk": "ukr_Cyrl",
    "hu": "hun_Latn",
    "ro": "ron_Latn",
    "bg": "bul_Cyrl",
    "fa": "pes_Arab",
    "ur": "urd_Arab",
    "tl": "tgl_Latn",
    "sw": "swh_Latn",
    "ca": "cat_Latn",
    "hr": "hrv_Latn",
    "sr": "srp_Cyrl",
    "sk": "slk_Latn",
    "sl": "slv_Latn",
    "et": "est_Latn",
    "lv": "lvs_Latn",
    "lt": "lit_Latn",
    "ms": "zsm_Latn",
    "ml": "mal_Mlym",
    "mr": "mar_Deva",
    "ta": "tam_Taml",
    "te": "tel_Telu",
    "kn": "kan_Knda",
    "gu": "guj_Gujr",
    "cy": "cym_Latn",
}

def get_nllb_lang_code(lang_code):
    if not lang_code:
        return None
    # If already in NLLB format, return as is
    if "_" in lang_code and len(lang_code) == 8:
        return lang_code
    return NLLB_LANG_MAP.get(lang_code.lower())

def translate_sliding_window(segments, engine, tokenizer, device, batch_size=8, src_lang=None, tgt_lang=None, ct2_tgt_prefix=None):
    """
    Translates segments using a sliding window approach: [Prev] + [Target] + [Next].
    This provides context to the model.
    """
    final_translations = [None] * len(segments)
    
    use_ct2 = hasattr(engine, "translate_batch")
    is_nllb = hasattr(engine, "config") and "nllb" in engine.config._name_or_path.lower() if not use_ct2 else (ct2_tgt_prefix is not None)

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
                # CTranslate2: Use configured batch size
                batch_size_ct2 = batch_size
                decoded = []
                for i in range(0, len(valid_inputs), batch_size_ct2):
                    mini_batch = valid_inputs[i:i + batch_size_ct2]
                    source_tokens = [tokenizer.convert_ids_to_tokens(tokenizer.encode(t, add_special_tokens=True)) for t in mini_batch]
                    target_prefixes = [ct2_tgt_prefix] * len(source_tokens) if ct2_tgt_prefix else None
                    # Use greedy search (beam_size=1) for NLLB to improve speed significantly
                    results = engine.translate_batch(source_tokens, target_prefix=target_prefixes, beam_size=1 if is_nllb else 2, asynchronous=False)
                    decoded.extend([tokenizer.decode(tokenizer.convert_tokens_to_ids(r.hypotheses[0]), skip_special_tokens=True) for r in results])
            else:
                # Transformers: Robust multi-line generation
                inputs = tokenizer(valid_inputs, return_tensors="pt", padding=True, truncation=True).to(device)
                with torch.inference_mode():
                    if is_nllb and tgt_lang:
                        # NLLB needs forced_bos_token_id for target language
                        tgt_lang_id = tokenizer.convert_tokens_to_ids(tgt_lang)
                        # Use greedy search (num_beams=1) for NLLB speed optimization
                        generated = engine.generate(**inputs, forced_bos_token_id=tgt_lang_id, num_beams=1, do_sample=False, use_cache=True)
                    else:
                        generated = engine.generate(**inputs, use_cache=True)
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
                    results = engine.translate_batch([source_tokens], target_prefix=[ct2_tgt_prefix] if ct2_tgt_prefix else None, asynchronous=False)
                    output_text = tokenizer.decode(tokenizer.convert_tokens_to_ids(results[0].hypotheses[0]), skip_special_tokens=True)
                    decoded.append(output_text)
            else:
                inputs = tokenizer(batch_texts, return_tensors="pt", padding=True, truncation=True).to(device)
                with torch.inference_mode():
                    if is_nllb and tgt_lang:
                        tgt_lang_id = tokenizer.convert_tokens_to_ids(tgt_lang)
                        generated = engine.generate(**inputs, forced_bos_token_id=tgt_lang_id, num_beams=1, do_sample=False, use_cache=True)
                    else:
                        generated = engine.generate(**inputs, use_cache=True)
                decoded = tokenizer.batch_decode(generated, skip_special_tokens=True)
            
            for ix, trans in zip(batch_ixs, decoded):
                final_translations[ix] = trans.strip()
                
    return final_translations

def translate_bulk(segments, engine, tokenizer, device, batch_size=16, src_lang=None, tgt_lang=None, ct2_tgt_prefix=None):
    """
    Standard batch translation.
    """
    use_ct2 = hasattr(engine, "translate_batch")
    is_nllb = hasattr(engine, "config") and "nllb" in engine.config._name_or_path.lower() if not use_ct2 else (ct2_tgt_prefix is not None)
    translated = []
    
    if use_ct2:
        # CTranslate2: Use configured batch size
        batch_size_ct2 = batch_size
        for i in range(0, len(segments), batch_size_ct2):
            batch = segments[i : i + batch_size_ct2]
            batch_clean = [s if s.strip() else " " for s in batch]
            source_tokens = [tokenizer.convert_ids_to_tokens(tokenizer.encode(t, add_special_tokens=True)) for t in batch_clean]
            target_prefixes = [ct2_tgt_prefix] * len(source_tokens) if ct2_tgt_prefix else None
            results = engine.translate_batch(source_tokens, target_prefix=target_prefixes, asynchronous=False)
            batch_decoded = [tokenizer.decode(tokenizer.convert_tokens_to_ids(r.hypotheses[0]), skip_special_tokens=True).strip() for r in results]
            
            for j, text in enumerate(batch):
                if not text.strip():
                    batch_decoded[j] = ""
            translated.extend(batch_decoded)
    else:
        for i in range(0, len(segments), batch_size):
            batch = segments[i : i + batch_size]
            batch_clean = [s if s.strip() else " " for s in batch] 
            inputs = tokenizer(batch_clean, return_tensors="pt", padding=True, truncation=True).to(device)
            with torch.inference_mode():
                if is_nllb and tgt_lang:
                    tgt_lang_id = tokenizer.convert_tokens_to_ids(tgt_lang)
                    generated = engine.generate(**inputs, forced_bos_token_id=tgt_lang_id, num_beams=1, do_sample=False, use_cache=True)
                else:
                    generated = engine.generate(**inputs, use_cache=True)
            decoded = tokenizer.batch_decode(generated, skip_special_tokens=True)
            
            for j, text in enumerate(batch):
                if not text.strip():
                    decoded[j] = ""
            translated.extend(decoded)
            
    return translated

def translate_with_sentence_splitting(segments, engine, tokenizer, device, batch_size=16, src_lang=None, tgt_lang=None, ct2_tgt_prefix=None):
    """
    Translates segments by splitting them into individual sentences for Helsinki models
    which often struggle with multi-sentence inputs.
    """
    all_sentences = []
    segment_map = [] # stores number of sentences per original segment
    
    sentence_splitter = re.compile(r'(?<=[.!?])\s+')
    
    for seg in segments:
        text = seg.strip()
        if not text:
            segment_map.append(0)
            continue
            
        sents = [s.strip() for s in sentence_splitter.split(text) if s.strip()]
        segment_map.append(len(sents))
        all_sentences.extend(sents)
        
    if not all_sentences:
        return ["" for _ in segments]
        
    # Use standard bulk translation for the flat list of sentences
    translated_sents = translate_bulk(all_sentences, engine, tokenizer, device, batch_size, src_lang, tgt_lang, ct2_tgt_prefix)
    
    final_results = []
    curr = 0
    for count in segment_map:
        if count == 0:
            final_results.append("")
            continue
        
        segment_translation = " ".join(translated_sents[curr : curr + count])
        final_results.append(segment_translation)
        curr += count
        
    return final_results

if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--model-path", required=True)
    parser.add_argument("--text", required=True)
    parser.add_argument("--mode", choices=["transcript", "document"], default="transcript")
    parser.add_argument("--src-lang", help="Source language code (e.g. en, eng_Latn)")
    parser.add_argument("--tgt-lang", help="Target language code (e.g. ja, jpn_Jpan)")
    
    # Advanced Configuration Arguments
    parser.add_argument("--batch-size-helsinki", type=int, help="Override batch size for Helsinki models")
    parser.add_argument("--batch-size-nllb", type=int, help="Override batch size for NLLB models")
    parser.add_argument("--threads", type=int, help="Override number of CPU threads")
    parser.add_argument("--device-preference", choices=["auto", "cpu", "cuda", "mps"], default="auto", help="Device preference")
    
    args = parser.parse_args()

    try:
        os.environ["HF_HUB_OFFLINE"] = "1"
        
        # Optimize CPU thread count for Intel/AMD
        cpu_count = os.cpu_count() or 4
        optimal_threads = max(1, int(cpu_count / 2))
        optimal_threads = min(optimal_threads, 8) 
        
        # Use override if provided
        if args.threads:
            optimal_threads = args.threads
            sys.stderr.write(f"[Python Debug] Overriding CPU threads to {optimal_threads}\n")
        
        torch.set_num_threads(optimal_threads)
        
        if sys.platform == "win32":
            sys.stdout.reconfigure(encoding='utf-8')
            
        # Determine device based on preference and availability
        device = "cpu"
        pref = args.device_preference
        
        if pref == "cuda":
            if torch.cuda.is_available(): device = "cuda"
            else: sys.stderr.write("[Python Debug] CUDA preferred but not available. Falling back to CPU.\n")
        elif pref == "mps":
            if torch.backends.mps.is_available(): device = "mps"
            else: sys.stderr.write("[Python Debug] MPS preferred but not available. Falling back to CPU.\n")
        elif pref == "cpu":
            device = "cpu"
        else: # auto
            if torch.backends.mps.is_available():
                device = "mps"
            elif torch.cuda.is_available():
                device = "cuda"
        
        # Heuristic to check for NLLB
        is_nllb = "nllb" in args.model_path.lower()
        
        # Determine language codes for NLLB
        nllb_src = get_nllb_lang_code(args.src_lang) if is_nllb else None
        nllb_tgt = get_nllb_lang_code(args.tgt_lang) if is_nllb else None

        sys.stderr.write(f"[Python Debug] Loading model from {args.model_path} (is_nllb={is_nllb}, device={device})...\n")
        
        # Check for CTranslate2 optimized model
        ct2_model_path = os.path.join(args.model_path, "ct2_optimized")
        has_ct2_model = os.path.exists(ct2_model_path)
        
        # Determine Backend (Auto)
        # Optimization for macOS (MPS):
        prefer_mps_for_nllb = torch.backends.mps.is_available() and is_nllb
        use_ct2 = has_ct2_model and not prefer_mps_for_nllb
        # If user forced 'mps' device, CT2 won't work, so use_ct2 should be false if device is mps.
        if device == "mps": use_ct2 = False
        
        engine = None
        if use_ct2:
            try:
                import ctranslate2
                
                # Determine CT2 device
                ct2_device = "cpu"
                if device == "cuda" and ctranslate2.get_cuda_device_count() > 0:
                    ct2_device = "cuda"
                
                sys.stderr.write(f"[Python Debug] Using CTranslate2 optimized engine: {ct2_model_path} (Device: {ct2_device})\n")
                # CT2 can use multiple threads. Use optimal_threads calculated above for CPU.
                engine = ctranslate2.Translator(ct2_model_path, device=ct2_device, intra_threads=optimal_threads if ct2_device == "cpu" else 0)
                
                if is_nllb:
                    tokenizer = AutoTokenizer.from_pretrained(args.model_path, src_lang=nllb_src)
                else:
                    tokenizer = MarianTokenizer.from_pretrained(args.model_path)
            except ImportError:
                sys.stderr.write("[Python Debug] CTranslate2 not found, falling back to transformers.\n")
                use_ct2 = False

        if not use_ct2:
            sys.stderr.write("[Python Debug] Loading with transformers engine.\n")
            if is_nllb:
                tokenizer = AutoTokenizer.from_pretrained(args.model_path, src_lang=nllb_src)
                # Use optimal dtype and low memory usage for NLLB
                engine = AutoModelForSeq2SeqLM.from_pretrained(
                    args.model_path,
                    torch_dtype=torch.float16 if device != "cpu" else torch.float32,
                    low_cpu_mem_usage=True
                )
                engine = engine.to(device).eval()
            else:
                tokenizer = AutoTokenizer.from_pretrained(args.model_path)
                engine = AutoModelForSeq2SeqLM.from_pretrained(
                    args.model_path,
                    torch_dtype=torch.float16 if device != "cpu" else torch.float32,
                    low_cpu_mem_usage=True
                ).to(device).eval()

        segments = json.loads(args.text)
        
        # Determine optimal batch size
        batch_size = 8

        # Helper to decide default based on model and device
        def get_auto_batch_size(is_nllb, device):
            if is_nllb:
                # NLLB is heavy: Keep CPU batch size low (1) to prevent hanging/OOM.
                # MPS/CUDA: Increase batch size to utilize GPU parallelism effectively.
                return 12 if device in ["cuda", "mps"] else 1
            else:
                # Helsinki is light
                return 32 if device in ["cuda", "mps"] else 16

        if is_nllb:
            if args.batch_size_nllb and args.batch_size_nllb > 0:
                batch_size = args.batch_size_nllb
            else:
                batch_size = get_auto_batch_size(True, device)
        else:
            if args.batch_size_helsinki and args.batch_size_helsinki > 0:
                batch_size = args.batch_size_helsinki
            else:
                batch_size = get_auto_batch_size(False, device)

        sys.stderr.write(f"[Python Debug] Using Batch Size: {batch_size}\n")
        
        results = []
        # For NLLB in CT2, we need to pass the target language prefix
        ct2_tgt_prefix = [nllb_tgt] if use_ct2 and is_nllb and nllb_tgt else None

        if args.mode == "document":
            sys.stderr.write(f"[Python Debug] Translating {len(segments)} segments in Document mode (bulk)...")
            results = translate_bulk(segments, engine, tokenizer, device, batch_size=batch_size, src_lang=nllb_src, tgt_lang=nllb_tgt, ct2_tgt_prefix=ct2_tgt_prefix)
        else:
            if is_nllb:
                sys.stderr.write(f"[Python Debug] Translating {len(segments)} segments in Transcript mode (sliding window)...")
                results = translate_sliding_window(segments, engine, tokenizer, device, batch_size=batch_size, src_lang=nllb_src, tgt_lang=nllb_tgt, ct2_tgt_prefix=ct2_tgt_prefix)
            else:
                sys.stderr.write(f"[Python Debug] Translating {len(segments)} segments in Transcript mode (sentence splitting)...")
                results = translate_with_sentence_splitting(segments, engine, tokenizer, device, batch_size=batch_size, src_lang=None, tgt_lang=None, ct2_tgt_prefix=None)
            
        print(json.dumps(results, ensure_ascii=False))

    except Exception as e:
        sys.stderr.write(f"Translation Error: {e}\n")
        import traceback
        traceback.print_exc(file=sys.stderr)
        sys.exit(1)