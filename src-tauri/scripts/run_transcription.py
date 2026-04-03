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

def run_transcription(audio_path, model_path, language=None, task="transcribe", device="auto", threads=None, compute_type_arg=None, beam_size=5, prompt=None, hotwords=None):
    # Compute compute_type based on device or argument
    compute_type = "int8"

    if compute_type_arg:
        compute_type = compute_type_arg
    elif device == "cuda":
        compute_type = "float16"

    # Load and run transcription
    try:
        # logging.info(f"Loading model from {model_path} on {device} with {compute_type}")
        model = WhisperModel(model_path, device=device, compute_type=compute_type, cpu_threads=threads if threads else 4)

        # Transcribe
        transcribe_args = {
            "audio": audio_path,
            "language": language,
            "task": task,
            "beam_size": beam_size,
            "word_timestamps": True
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
            if segment.words:
                for w in segment.words:
                    segment_words.append({
                        "start": w.start,
                        "end": w.end,
                        "text": w.word.strip(),
                        "probability": w.probability
                    })

            results.append({
                "start": segment.start,
                "end": segment.end,
                "text": segment.text.strip(),
                "speaker": "Unknown",
                "words": segment_words
            })

        print(json.dumps({"segments": results}), flush=True)

    except Exception as e:
        print(json.dumps({"error": f"Transcription failed: {str(e)}"}), flush=True)

if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--audio", required=True)
    parser.add_argument("--model", required=True)
    parser.add_argument("--language", default=None)
    parser.add_argument("--task", default="transcribe")
    parser.add_argument("--device", default="auto")
    parser.add_argument("--threads", type=int, default=4)
    parser.add_argument("--compute_type", default=None, help="int8, float16, int8_float16")
    parser.add_argument("--beam_size", type=int, default=5)
    parser.add_argument("--prompt", type=str, default=None, help="Initial prompt to guide the model")
    parser.add_argument("--hotwords", type=str, default=None, help="Hotwords for the model")

    args = parser.parse_args()

    # Check if language is "auto", convert to None
    lang = args.language
    if lang == "auto":
        lang = None

    run_transcription(args.audio, args.model, lang, args.task, args.device, args.threads, args.compute_type, args.beam_size, args.prompt, args.hotwords)
