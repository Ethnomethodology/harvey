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

def run_transcription(audio_path, model_path, language=None, task="transcribe", device="auto", threads=None):
    # Compute compute_type based on device
    # For CPU, int8 is faster. For GPU, float16 is standard.
    compute_type = "int8"
    if device == "cuda":
        compute_type = "float16"

    # Load model
    try:
        # logging.info(f"Loading model from {model_path} on {device} with {compute_type}")
        model = WhisperModel(model_path, device=device, compute_type=compute_type, cpu_threads=threads if threads else 4)
    except Exception as e:
        print(json.dumps({"error": f"Failed to load model: {str(e)}"}), flush=True)
        return

    # Transcribe
    try:
        # logging.info(f"Transcribing {audio_path} with language={language} task={task}")
        segments, info = model.transcribe(audio_path, language=language, task=task, beam_size=5)

        results = []
        for segment in segments:
            results.append({
                "start": segment.start,
                "end": segment.end,
                "text": segment.text.strip(),
                "speaker": "Unknown"
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

    args = parser.parse_args()

    # Check if language is "auto", convert to None
    lang = args.language
    if lang == "auto":
        lang = None

    run_transcription(args.audio, args.model, lang, args.task, args.device, args.threads)
