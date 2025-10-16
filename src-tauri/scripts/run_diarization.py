# src-tauri/scripts/run_diarization.py
import argparse
import os
import sys
import torch
from pyannote.audio import Pipeline

def run_diarization(audio_path, num_speakers, token):
    """
    Runs speaker diarization on an audio file using pyannote.audio.

    Args:
        audio_path (str): Path to the audio file.
        num_speakers (int): Number of speakers. If 0, the model will detect the number.
        token (str): Hugging Face authentication token.
    """
    try:
        if not os.path.exists(audio_path):
            print(f"Error: Audio file not found at {audio_path}", file=sys.stderr)
            sys.exit(1)

        # Check for GPU availability
        device = "cuda" if torch.cuda.is_available() else "cpu"
        print(f"Using device: {device}", file=sys.stderr)

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

    args = parser.parse_args()

    run_diarization(args.audio_path, args.num_speakers, args.token)