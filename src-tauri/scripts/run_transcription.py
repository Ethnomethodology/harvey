import sys
import json
import logging
import argparse
# faster_whisper should be installed in the environment
try:
    from faster_whisper import WhisperModel
except ImportError as e:
    print(json.dumps({"error": f"faster_whisper import failed: {str(e)}"}), flush=True)
    sys.exit(1)

# Configure logging to stderr to avoid polluting stdout which is used for JSON output
logging.basicConfig(level=logging.INFO, stream=sys.stderr)

def run_transcription(audio_path, model_path, language=None, task="transcribe", device="auto", threads=None, compute_type_arg=None, beam_size=5, prompt=None, hotwords=None, without_timestamps=False):
    # Compute compute_type based on device or argument
    compute_type = "int8"
    if compute_type_arg is not None:
        compute_type = compute_type_arg
    elif device == "cuda":
        compute_type = "float16"

    try:
        # logging.info(f"Loading model from {model_path} on {device} with {compute_type}")
        model = WhisperModel(model_path, device=device, compute_type=compute_type, cpu_threads=threads if threads else 4)

        is_crisper = "CrisperWhisper" in model_path
        
        # Enforce beam_size=1 for CrisperWhisper as recommended by the authors for performance on this large-v3 model
        if is_crisper:
            beam_size = 1

        # Transcribe

        transcribe_args = {
            "audio": audio_path,
            "language": language,
            "task": task,
            "beam_size": beam_size,
            "word_timestamps": True,
            "without_timestamps": without_timestamps,
            "condition_on_previous_text": False
        }

        if prompt is not None:
            transcribe_args["initial_prompt"] = prompt

        if hotwords is not None:
            transcribe_args["hotwords"] = hotwords

        # logging.info(f"Transcribing {audio_path} with language={language} task={task}")
        segments, info = model.transcribe(**transcribe_args)
        
        results = []
        for segment in segments:
            segment_words = []
            segment_text = segment.text
            if is_crisper:
                # CrisperWhisper's custom tokenizer outputs commas instead of spaces
                segment_text = segment_text.replace(',', ' ')
                # It also drops spaces after punctuation, so we restore them
                import re
                segment_text = re.sub(r'([.?!])([a-zA-Z])', r'\1 \2', segment_text).strip()
                
            if segment.words:
                for w in segment.words:
                    word_text = w.word
                    if is_crisper:
                        word_text = word_text.replace(',', ' ')
                        word_text = re.sub(r'([.?!])([a-zA-Z])', r'\1 \2', word_text).strip()
                    else:
                        word_text = word_text.strip()
                        
                    segment_words.append({
                        "start": w.start,
                        "end": w.end,
                        "text": word_text,
                        "probability": w.probability
                    })
            results.append({
                "start": segment.start,
                "end": segment.end,
                "text": segment_text,
                "speaker": "Unknown",
                "words": segment_words
            })

        print(json.dumps({"segments": results}))

    except Exception as e:
        print(json.dumps({"error": f"Transcription failed: {str(e)}"}))
        sys.exit(1)

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Run transcription with faster-whisper")
    parser.add_argument("--audio", type=str, required=True, help="Path to the audio file")
    parser.add_argument("--model", type=str, required=True, help="Path to the model directory")
    parser.add_argument("--language", type=str, default="auto", help="Language code or 'auto'")
    parser.add_argument("--task", type=str, default="transcribe", choices=["transcribe", "translate"], help="Task to perform")
    parser.add_argument("--device", type=str, default="auto", choices=["auto", "cpu", "cuda"], help="Device to use")
    parser.add_argument("--threads", type=int, default=None, help="Number of CPU threads")
    parser.add_argument("--compute_type", type=str, default=None, help="Compute type (int8, float16, etc.)")
    parser.add_argument("--beam_size", type=int, default=5, help="Beam size")
    parser.add_argument("--prompt", type=str, default=None, help="Initial prompt to guide the model")
    parser.add_argument("--hotwords", type=str, default=None, help="Hotwords for the model")
    parser.add_argument("--without_timestamps", action="store_true", help="Disable timestamps output for verbatim models")

    args = parser.parse_args()

    # Check if language is "auto", convert to None
    lang = args.language
    if lang == "auto":
        lang = None

    run_transcription(args.audio, args.model, lang, args.task, args.device, args.threads, args.compute_type, args.beam_size, args.prompt, args.hotwords, args.without_timestamps)
