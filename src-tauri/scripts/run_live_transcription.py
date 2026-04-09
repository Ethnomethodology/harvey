import sys
import json
import logging
import argparse
import time
import numpy as np
import collections
import wave
import os
import threading

# Configure logging to stderr
logging.basicConfig(level=logging.INFO, stream=sys.stderr)

try:
    from faster_whisper import WhisperModel
    import sounddevice as sd
except ImportError as e:
    print(json.dumps({"error": f"Import failed: {str(e)}"}), flush=True)
    sys.exit(1)


def run_live_transcription(model_path, language=None, device="auto", threads=4,
                           step_ms=5000, length_ms=5000, save_audio_path=None):
    """
    Live transcription using faster-whisper and sounddevice.
    Audio capture is done via sounddevice (PortAudio) which inherits the macOS
    TCC microphone entitlement from the parent Harvey process.
    """
    # --- Signal UI immediately so it shows "Initializing..." correctly ---
    # Model loading can take 30-60 seconds. We print this early so the Rust
    # side can emit live_transcription_ready only once the model is actually
    # ready. We use a different sentinel here to distinguish "loading" vs "ready".
    logging.info("[Live Transcription] Loading model...")

    compute_type = "int8"
    if device == "cuda":
        compute_type = "float16"

    try:
        model = WhisperModel(model_path, device=device, compute_type=compute_type, cpu_threads=threads)
    except Exception as e:
        print(json.dumps({"error": f"Failed to load model: {str(e)}"}), flush=True)
        sys.exit(1)

    sample_rate = 16000
    chunk_samples = int(sample_rate * (step_ms / 1000.0))

    # Rolling buffer: keep the last `length_ms` ms of audio
    buffer_maxlen = int(sample_rate * (length_ms / 1000.0))
    audio_buffer = collections.deque(maxlen=buffer_maxlen)
    buffer_lock = threading.Lock()

    # Open WAV file for saving if requested (opened after stream is confirmed open)
    wav_file = None

    # --- Pre-flight: check that a microphone is actually available ---
    try:
        default_input = sd.query_devices(kind='input')
        max_ch = default_input.get('max_input_channels', 0) if default_input else 0
        if max_ch == 0:
            print(json.dumps({"error": (
                "Microphone access denied or no input device found. "
                "Please allow Harvey to access your microphone in "
                "System Settings > Privacy & Security > Microphone."
            )}), flush=True)
            sys.exit(1)
        logging.info(f"[Live Transcription] Input device: {default_input.get('name')} ({max_ch} ch)")
    except Exception as e:
        print(json.dumps({"error": f"Microphone check failed: {str(e)}"}), flush=True)
        sys.exit(1)

    def audio_callback(indata, frames, time_info, status):
        if status:
            logging.warning(f"[Audio] Status: {status}")
        with buffer_lock:
            audio_buffer.extend(indata[:, 0])
        if wav_file:
            audio_int16 = (indata[:, 0] * 32767).astype(np.int16)
            wav_file.writeframes(audio_int16.tobytes())

    try:
        stream = sd.InputStream(
            samplerate=sample_rate,
            channels=1,
            callback=audio_callback,
            blocksize=chunk_samples
        )
        stream.start()
    except Exception as e:
        err = str(e)
        if "Invalid number of channels" in err or "unauthenticated" in err.lower():
            print(json.dumps({"error": (
                "Microphone access denied. Please allow Harvey to access the microphone "
                "in System Settings > Privacy & Security > Microphone."
            )}), flush=True)
        else:
            print(json.dumps({"error": f"Failed to open audio stream: {err}"}), flush=True)
        sys.exit(1)

    # Open WAV file *after* stream confirmed open (so we only create it if recording works)
    if save_audio_path:
        try:
            wav_file = wave.open(save_audio_path, 'wb')
            wav_file.setnchannels(1)
            wav_file.setsampwidth(2)
            wav_file.setframerate(sample_rate)
            logging.info(f"[Live Transcription] Saving audio to {save_audio_path}")
        except Exception as e:
            logging.error(f"[Live Transcription] Failed to open WAV file: {e}")

    # Signal Rust/frontend that we are ready
    print("[Start speaking]", flush=True)
    logging.info("[Live Transcription] Ready — microphone open, transcribing.")

    try:
        while True:
            time.sleep(step_ms / 1000.0)

            with buffer_lock:
                if len(audio_buffer) < chunk_samples:
                    continue
                audio_data = np.array(list(audio_buffer), dtype=np.float32)

            segments, info = model.transcribe(
                audio_data,
                language=language,
                beam_size=5,
                vad_filter=True,
                vad_parameters=dict(min_silence_duration_ms=500)
            )

            full_text = "".join(seg.text for seg in segments).strip()
            if full_text:
                print(full_text, flush=True)

    except KeyboardInterrupt:
        pass
    except Exception as e:
        print(json.dumps({"error": f"Transcription loop error: {str(e)}"}), flush=True)
    finally:
        stream.stop()
        stream.close()
        if wav_file:
            wav_file.close()
        logging.info("[Live Transcription] Stopped.")


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--model", required=True)
    parser.add_argument("--language", default=None)
    parser.add_argument("--device", default="auto")
    parser.add_argument("--threads", type=int, default=4)
    parser.add_argument("--step", type=int, default=5000)
    parser.add_argument("--length", type=int, default=5000)
    parser.add_argument("--save-audio", default=None,
                        help="Full path to save audio WAV file")

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
        args.length,
        args.save_audio,
    )
