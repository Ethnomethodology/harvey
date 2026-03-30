import sys
import json
import logging
import argparse
import time
import numpy as np
import collections
import wave
import os

# Configure logging to stderr
logging.basicConfig(level=logging.INFO, stream=sys.stderr)

try:
    from faster_whisper import WhisperModel
    import sounddevice as sd
except ImportError as e:
    print(json.dumps({"error": f"Import failed: {str(e)}"}), flush=True)
    sys.exit(1)

def run_live_transcription(model_path, language=None, device="auto", threads=4, step_ms=5000, length_ms=5000, save_audio=False):
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
    
    wav_file = None
    if save_audio:
        timestamp = time.strftime("%Y%m%d%H%M%S")
        filename = f"{timestamp}.wav"
        wav_file = wave.open(filename, 'wb')
        wav_file.setnchannels(1)
        wav_file.setsampwidth(2) # 16-bit
        wav_file.setframerate(sample_rate)
        logging.info(f"Saving audio to {os.path.abspath(filename)}")

    print("[Start speaking]", flush=True)

    def audio_callback(indata, frames, time, status):
        if status:
            logging.warning(status)
        audio_buffer.extend(indata[:, 0])
        if wav_file:
            # Convert float32 to int16 for WAV saving
            audio_int16 = (indata[:, 0] * 32767).astype(np.int16)
            wav_file.writeframes(audio_int16.tobytes())

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
                        vad_parameters=dict(min_silence_duration_ms=500),
                        word_timestamps=True
                    )

                    for segment in segments:
                        for word in segment.words:
                            text = word.word.strip()
                            if text:
                                # We print word by word. To mimic whisper-stream,
                                # we print the text and flush.
                                print(text, flush=True)
                                # time.sleep to simulate typing if necessary, but
                                # since it's live, we just print as fast as the model predicts

                time.sleep(step_ms / 2000.0) # Sleep for half the step size

    except KeyboardInterrupt:
        pass
    except Exception as e:
        print(json.dumps({"error": f"Live transcription failed: {str(e)}"}), flush=True)
    finally:
        if wav_file:
            wav_file.close()

if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--model", required=True)
    parser.add_argument("--language", default=None)
    parser.add_argument("--device", default="auto")
    parser.add_argument("--threads", type=int, default=4)
    parser.add_argument("--step", type=int, default=5000)
    parser.add_argument("--length", type=int, default=5000)
    parser.add_argument("--save-audio", action="store_true")

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
        args.save_audio
    )
