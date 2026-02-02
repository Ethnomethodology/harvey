# src-tauri/scripts/run_diarization.py
import argparse
import os
import sys
import torch
from pyannote.audio import Pipeline

def run_diarization(audio_path, num_speakers, token, device_pref=None, threads_pref=None):
    """
    Runs speaker diarization on an audio file using pyannote.audio.
    """
    try:
        if not os.path.exists(audio_path):
            print(f"Error: Audio file not found at {audio_path}", file=sys.stderr)
            sys.exit(1)

        # Force offline mode
        os.environ["HF_HUB_OFFLINE"] = "1"

        # 1. Optimize CPU threads (Intel/AMD optimization)
        # Prevent PyTorch from using all cores which can cause contention.
        # Heuristic: Use ~physical cores, capped at 8.
        cpu_count = os.cpu_count() or 4
        optimal_threads = max(1, int(cpu_count / 2))
        optimal_threads = min(optimal_threads, 8)
        
        # Override threads if provided
        if threads_pref:
            optimal_threads = threads_pref
            
        torch.set_num_threads(optimal_threads)

        # 2. Hardware Acceleration Detection (CUDA > MPS > CPU)
        device = "cpu"
        
        if device_pref and device_pref != "auto":
            if device_pref == "cuda":
                if torch.cuda.is_available(): device = "cuda"
                else: print("Warning: CUDA requested but not available. Falling back to CPU.", file=sys.stderr)
            elif device_pref == "mps":
                if torch.backends.mps.is_available(): device = "mps"
                else: print("Warning: MPS requested but not available. Falling back to CPU.", file=sys.stderr)
            elif device_pref == "cpu":
                device = "cpu"
        else:
            if torch.cuda.is_available():
                device = "cuda"
            elif torch.backends.mps.is_available():
                device = "mps"
            else:
                device = "cpu"
            
        print(f"Using device: {device} (Threads: {optimal_threads})", file=sys.stderr)

        # Fix for PyTorch 2.6+ weights_only=True default causing unpickling errors with Pyannote/SpeechBrain
        try:
            # Attempt to allowlist TorchVersion and Pyannote Specifications
            from torch.torch_version import TorchVersion
            from pyannote.audio.core.task import Specifications, Problem, Resolution
            torch.serialization.add_safe_globals([TorchVersion, Specifications, Problem, Resolution])
        except (ImportError, AttributeError, Exception) as e:
            print(f"Warning: Failed to add safe globals: {e}", file=sys.stderr)
            pass

        # Set auth token in environment variables since Pipeline.from_pretrained might not accept it directly
        if token:
            os.environ["HF_TOKEN"] = token
            os.environ["HUGGING_FACE_HUB_TOKEN"] = token

        pipeline = Pipeline.from_pretrained(
            "pyannote/speaker-diarization-3.1"
        ).to(torch.device(device))

        if num_speakers > 0:
            diarization = pipeline(audio_path, num_speakers=num_speakers)
        else:
            # If num_speakers is 0, let the model determine the number automatically
            diarization = pipeline(audio_path)

        # Output the RTTM content to stdout
        diarization.speaker_diarization.write_rttm(sys.stdout)

    except Exception as e:
        print(f"An error occurred during diarization: {e}", file=sys.stderr)
        sys.exit(1)

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Run speaker diarization using pyannote.audio.")
    parser.add_argument("audio_path", type=str, help="Path to the audio file.")
    parser.add_argument("num_speakers", type=int, help="Number of speakers (0 for automatic).")
    parser.add_argument("token", type=str, help="Hugging Face authentication token.")
    parser.add_argument("--device", choices=["auto", "cpu", "cuda", "mps"], default="auto", help="Device preference")
    parser.add_argument("--threads", type=int, help="Override number of CPU threads")

    args = parser.parse_args()

    run_diarization(args.audio_path, args.num_speakers, args.token, args.device, args.threads)