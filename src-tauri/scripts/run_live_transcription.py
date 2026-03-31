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

    try:
        from faster_whisper.vad import get_speech_timestamps, get_vad_model
        vad_model = get_vad_model()
    except Exception as e:
        logging.warning(f"Could not load VAD model: {e}")
        vad_model = None

    sample_rate = 16000
    # Process audio every `step_ms` (e.g. 1000ms)
    chunk_samples = int(sample_rate * (step_ms / 1000.0))
    # Maximum buffer size before forcing a finalization (e.g. 30 seconds)
    max_samples = sample_rate * 30
    
    audio_buffer = []
    
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
        audio_buffer.extend(indata[:, 0].tolist())
        if wav_file:
            audio_int16 = (indata[:, 0] * 32767).astype(np.int16)
            wav_file.writeframes(audio_int16.tobytes())

    try:
        with sd.InputStream(samplerate=sample_rate, channels=1, callback=audio_callback, blocksize=chunk_samples):
            while True:
                if len(audio_buffer) >= chunk_samples:
                    # Work on a copy of the buffer
                    current_buffer = list(audio_buffer)
                    audio_data = np.array(current_buffer, dtype=np.float32)
                    
                    is_final = False

                    # If the buffer is getting too large, force finalization
                    if len(audio_data) >= max_samples:
                        is_final = True
                    # Else, check if the user stopped speaking using VAD
                    elif vad_model:
                        try:
                            speech_timestamps = get_speech_timestamps(audio_data, vad_model, sampling_rate=sample_rate)
                            if speech_timestamps:
                                # Get end of last speech segment
                                last_speech_end = speech_timestamps[-1]['end']
                                silence_tail = len(audio_data) - last_speech_end
                                # If there is > 1.0s of silence at the end, finalize
                                if silence_tail > sample_rate * 1.0:
                                    is_final = True
                        except Exception as e:
                            logging.debug(f"VAD error: {e}")

                    # Transcribe current buffer
                    try:
                        segments, info = model.transcribe(
                            audio_data,
                            language=language,
                            beam_size=5,
                            vad_filter=True,
                            vad_parameters=dict(min_silence_duration_ms=500),
                            condition_on_previous_text=False
                        )

                        full_text = ""
                        for segment in segments:
                            full_text += segment.text

                        text = full_text.strip()
                        if text:
                            # Append '...' if interim to tell Rust `is_final = False`
                            output_text = text if is_final else text + "..."
                            print(output_text, flush=True)

                        if is_final:
                            # Clear buffer up to the transcribed amount
                            # Keep a small overlap (0.5s) to avoid missing words at boundaries
                            overlap = int(sample_rate * 0.5)
                            if len(audio_buffer) > len(current_buffer):
                                # Audio arrived while transcribing
                                audio_buffer = current_buffer[-overlap:] + audio_buffer[len(current_buffer):]
                            else:
                                audio_buffer = current_buffer[-overlap:]
                    except Exception as e:
                        logging.error(f"Transcription error: {e}")

                time.sleep(step_ms / 1000.0)

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
