import sys
import json
import logging
import argparse
import time
import numpy as np
import collections

# Configure logging to stderr
logging.basicConfig(level=logging.INFO, stream=sys.stderr)

try:
    from faster_whisper import WhisperModel
    import sounddevice as sd
except ImportError as e:
    print(json.dumps({"error": f"Import failed: {str(e)}"}), flush=True)
    sys.exit(1)

def run_live_transcription(model_path, language=None, device="auto", threads=4, step_ms=5000, length_ms=5000):
    """
    Live transcription using faster-whisper and sounddevice.
    Mimics whisper-stream output format for Harvey integration.
    """
    compute_type = "int8"
    if device == "cuda":
        compute_type = "float16"

    try:
        model = WhisperModel(model_path, device=device, compute_type=compute_type, cpu_threads=threads)
    except Exception as e:
        print(json.dumps({"error": f"Failed to load model: {str(e)}"}), flush=True)
        return

    sample_rate = 16000
    chunk_samples = int(sample_rate * (step_ms / 1000.0))
    
    # Buffer to hold audio for transcription
    audio_buffer = collections.deque(maxlen=int(sample_rate * (length_ms / 1000.0)))
    
    print("[Start speaking]", flush=True)

    def audio_callback(indata, frames, time, status):
        if status:
            logging.warning(status)
        audio_buffer.extend(indata[:, 0])

    try:
        with sd.InputStream(samplerate=sample_rate, channels=1, callback=audio_callback, blocksize=chunk_samples):
            while True:
                if len(audio_buffer) >= chunk_samples:
                    # Convert buffer to numpy array
                    audio_data = np.array(list(audio_buffer), dtype=np.float32)
                    
                    # Transcribe
                    segments, info = model.transcribe(
                        audio_data, 
                        language=language, 
                        beam_size=5,
                        vad_filter=True,
                        vad_parameters=dict(min_silence_duration_ms=500)
                    )

                    full_text = ""
                    for segment in segments:
                        full_text += segment.text

                    text = full_text.strip()
                    if text:
                        # Harvey's Rust side expects a specific format or just text.
                        # whisper-stream typically prints text directly.
                        # We use '...' to signal a partial result if it's not the end of a sentence.
                        # For now, we'll keep it simple as whisper-stream does.
                        print(text, flush=True)

                time.sleep(step_ms / 2000.0) # Sleep for half the step size

    except KeyboardInterrupt:
        pass
    except Exception as e:
        print(json.dumps({"error": f"Live transcription failed: {str(e)}"}), flush=True)

if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--model", required=True)
    parser.add_argument("--language", default=None)
    parser.add_argument("--device", default="auto")
    parser.add_argument("--threads", type=int, default=4)
    parser.add_argument("--step", type=int, default=5000)
    parser.add_argument("--length", type=int, default=5000)

    args = parser.parse_args()

    lang = args.language
    if lang == "auto":
        lang = None

    run_live_transcription(
        args.model, 
        lang, 
        args.device, 
        args.threads, 
        args.step, 
        args.length
    )
