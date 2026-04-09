use crate::projectview::shared_types::TranscriptSegment;
use crate::welcome::config::CommandError;
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::sync::{atomic::AtomicBool, Arc};

pub mod faster_whisper;
pub mod faster_whisper_live;
pub mod python_engine;
pub mod whisper_cpp;

#[derive(Debug, Clone)]
pub struct TranscriptionOptions {
    pub language_code: Option<String>,
    pub model_path: String,
    pub output_dir: PathBuf,
    pub translate: bool,
    pub initial_prompt: Option<String>,
    pub hotwords: Option<String>,
}

#[async_trait]
pub trait TranscriptionEngine: Send + Sync {
    /// Transcribes the given audio file.
    /// Returns a vector of TranscriptSegments (start, end, text, speaker).
    async fn transcribe(
        &self,
        audio_path: &Path,
        options: &TranscriptionOptions,
        job_id: &str,
        cancel_flag: Arc<AtomicBool>,
    ) -> Result<Vec<TranscriptSegment>, CommandError>;
}

/// Merges consecutive short segments into larger chunks when diarization is disabled.
/// Strategy:
/// - Gap between segments < 1.0s
/// - Combined duration < 20.0s
/// - Combined character count < 500
/// - Respects sentence boundaries (ends with . ? !) unless the segment is very short (< 3s)
pub fn merge_smaller_segments(segments: Vec<TranscriptSegment>) -> Vec<TranscriptSegment> {
    if segments.len() <= 1 {
        return segments;
    }

    let original_count = segments.len();
    let mut merged: Vec<TranscriptSegment> = Vec::new();
    let mut iter = segments.into_iter();
    let mut current = match iter.next() {
        Some(s) => s,
        None => return merged,
    };

    for next in iter {
        let gap = next.start_time - current.end_time;
        let combined_duration = next.end_time - current.start_time;
        let current_text_trimmed = current.text.trim();
        let next_text_trimmed = next.text.trim();

        let combined_text_len = current_text_trimmed.len() + next_text_trimmed.len() + 1;

        // Syntactic check: does current segment end with a sentence terminator?
        let ends_with_terminator = current_text_trimmed.ends_with('.')
            || current_text_trimmed.ends_with('?')
            || current_text_trimmed.ends_with('!');

        let current_duration = current.end_time - current.start_time;

        // Decision logic:
        // 1. If gap is too large (> 1.0s), don't merge.
        // 2. If already too long (> 20s or > 500 chars), don't merge.
        // 3. If it ends with punctuation AND it's not "tiny" (< 3.0s), don't merge.

        let is_tiny = current_duration < 3.0;
        let small_gap = gap < 1.0;
        let reasonable_size = combined_duration < 20.0 && combined_text_len < 500;

        let should_merge = small_gap
            && reasonable_size
            && (!ends_with_terminator || is_tiny);

        if should_merge {
            // Combine text
            current.text = format!("{} {}", current_text_trimmed, next_text_trimmed);
            current.end_time = next.end_time;

            // Merge words if present
            if let Some(mut current_words) = current.words.take() {
                if let Some(next_words) = next.words {
                    current_words.extend(next_words);
                }
                current.words = Some(current_words);
            } else {
                current.words = next.words;
            }

            // Keep the earliest speaker if they happen to be different (shouldn't be if diarization is off)
            if current.speaker == "Unknown" || current.speaker.is_empty() {
                current.speaker = next.speaker;
            }
        } else {
            // Push current and move to next
            merged.push(current);
            current = next;
        }
    }

    merged.push(current);

    log::info!(
        "[Transcription] Merged {} segments into {} grouping-optimized segments.",
        original_count,
        merged.len()
    );

    merged
}

#[async_trait]
pub trait TranslationEngine: Send + Sync {
    /// Translates a list of text strings.
    /// Returns a list of translated strings in the same order.
    async fn translate(
        &self,
        texts: Vec<String>,
        model_path: &Path,
        job_id: &str,
        cancel_flag: Arc<AtomicBool>,
        mode: &str,
        src_lang: Option<&str>,
        tgt_lang: Option<&str>,
    ) -> Result<Vec<String>, CommandError>;
}
