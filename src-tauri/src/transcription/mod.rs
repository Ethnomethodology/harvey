use async_trait::async_trait;
use std::path::{Path, PathBuf};
use crate::welcome::config::CommandError;
use std::sync::{Arc, atomic::AtomicBool};
use crate::projectview::shared_types::TranscriptSegment;

pub mod whisper_cpp;
pub mod python_engine;

#[derive(Debug, Clone)]
pub struct TranscriptionOptions {
    pub language_code: Option<String>,
    pub model_path: String,
    pub output_dir: PathBuf,
    pub translate: bool,
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