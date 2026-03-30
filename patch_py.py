import sys

def modify_py_script(filepath):
    with open(filepath, 'r') as f:
        content = f.read()

    # We want to change model.transcribe(...) to include word_timestamps=True
    # and then iterate over words.

    old_transcribe = """
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
"""

    new_transcribe = """
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
"""

    content = content.replace(old_transcribe, new_transcribe)

    with open(filepath, 'w') as f:
        f.write(content)

modify_py_script("src-tauri/scripts/run_live_transcription.py")
