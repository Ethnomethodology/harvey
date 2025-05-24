// src-tauri/src/projectview/cloud_handler/cloud_transcribe.rs

// --- UPDATED: Imports reflecting new module structure ---
use crate::projectview::shared_types::{TranscriptSegment, TranscriptionResult}; // Shared types
use crate::projectview::shared_utils::{get_project_xml_path_from_item}; // Shared utils
use crate::projectview::transcription_commands::{ // Transcription-specific commands/helpers
    prepare_output_paths, save_transcript_json, map_speaker_ids_to_names, generate_lexical_doc,
};
use serde_json;
// Import local_handler helpers needed here
use crate::projectview::local_handler::transcription::{ // Helpers currently in local_handler
    convert_to_wav_if_needed, emit_progress,
};
use crate::welcome::config::CommandError;
use crate::TranscriptionCancellationState;
// --- END UPDATED IMPORTS ---

use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use log::{debug, error, info, warn};
use reqwest::Client as ReqwestClient;
use serde::{Deserialize, Serialize};
use std::{
    fs as std_fs,
    path::{PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};
use tauri::{AppHandle, State};
use tokio::{fs::File, io::AsyncReadExt, time::sleep};
use std::str::FromStr;


// --- Structs for Gemini API (Keep local) ---
#[derive(Serialize, Debug)] struct GeminiRequest { contents: Vec<GeminiContent> }
#[derive(Serialize, Deserialize, Debug, Clone)] struct GeminiContent { parts: Vec<GeminiPart> }
#[derive(Serialize, Deserialize, Debug, Clone)] struct GeminiPart { text: Option<String>, #[serde(rename = "inlineData", skip_serializing_if = "Option::is_none")] inline_data: Option<GeminiInlineData> }
#[derive(Serialize, Deserialize, Debug, Clone)] struct GeminiInlineData { #[serde(rename = "mimeType")] mime_type: String, data: String }
#[derive(Deserialize, Debug)] struct GeminiResponse { candidates: Option<Vec<GeminiCandidate>>, #[serde(rename = "promptFeedback")] prompt_feedback: Option<GeminiPromptFeedback> }
#[derive(Deserialize, Debug)] struct GeminiCandidate { content: Option<GeminiContent>, #[serde(rename = "finishReason")] finish_reason: Option<String> }
#[derive(Deserialize, Debug)] struct GeminiPromptFeedback { #[serde(rename = "blockReason")] block_reason: Option<String> }

// Struct for parsing the JSON array expected within Gemini's text response
#[derive(Deserialize, Debug, Clone)] struct GeminiJsonSegment {
    start_time: String,
    end_time: String,
    speaker_no: String,
    text: String,
}

// --- Helper: Parse hh:mm:ss.ms string to f64 seconds ---
fn parse_hhmmss_ms_to_seconds(timestamp_str: &str) -> Result<f64, CommandError> {
    let timestamp_str = timestamp_str.trim();
    let parts: Vec<&str> = timestamp_str.split(':').collect();

    let total_seconds = match parts.len() {
        3 => {
            let hours_str = parts[0];
            let minutes_str = parts[1];
            let seconds_ms_str = parts[2];

            let hours: u64 = hours_str.parse().map_err(|e| CommandError::from(format!("Invalid hours '{}' in timestamp '{}': {}", hours_str, timestamp_str, e)))?;
            let minutes: u64 = minutes_str.parse().map_err(|e| CommandError::from(format!("Invalid minutes '{}' in timestamp '{}': {}", minutes_str, timestamp_str, e)))?;

            let seconds_parts: Vec<&str> = seconds_ms_str.split(|c| c == '.' || c == ',').collect();
            let sec: u64 = seconds_parts.get(0).unwrap_or(&"")
                .parse().map_err(|e| CommandError::from(format!("Invalid seconds '{}' in timestamp '{}': {}", seconds_parts.get(0).unwrap_or(&""), timestamp_str, e)))?;
            let ms_str = seconds_parts.get(1).unwrap_or(&"0");
            let padded_ms_str = format!("{:0<3}", ms_str.chars().take(3).collect::<String>());
            let ms: u32 = padded_ms_str.parse().map_err(|e| CommandError::from(format!("Invalid milliseconds '{}' (from '{}') in timestamp '{}': {}", ms_str, padded_ms_str, timestamp_str, e)))?;

            (hours * 3600 + minutes * 60 + sec) as f64 + (ms as f64 / 1000.0)
        },
        2 => {
            let minutes_str = parts[0];
            let seconds_ms_str = parts[1];

            let minutes: u64 = minutes_str.parse().map_err(|e| CommandError::from(format!("Invalid minutes '{}' in timestamp '{}': {}", minutes_str, timestamp_str, e)))?;
            let seconds_parts: Vec<&str> = seconds_ms_str.split(|c| c == '.' || c == ',').collect();
            let sec: u64 = seconds_parts.get(0).unwrap_or(&"")
                .parse().map_err(|e| CommandError::from(format!("Invalid seconds '{}' in timestamp '{}': {}", seconds_parts.get(0).unwrap_or(&""), timestamp_str, e)))?;
            let ms_str = seconds_parts.get(1).unwrap_or(&"0");
            let padded_ms_str = format!("{:0<3}", ms_str.chars().take(3).collect::<String>());
            let ms: u32 = padded_ms_str.parse().map_err(|e| CommandError::from(format!("Invalid milliseconds '{}' (from '{}') in timestamp '{}': {}", ms_str, padded_ms_str, timestamp_str, e)))?;

            (minutes * 60 + sec) as f64 + (ms as f64 / 1000.0)
        },
        _ => {
             match f64::from_str(timestamp_str) {
                 Ok(float_seconds) => float_seconds,
                 Err(_) => {
                     let msg = format!("Timestamp '{}' is not in a recognized format (hh:mm:ss.ms, mm:ss.ms, or float seconds).", timestamp_str);
                     warn!("[Gemini Parse] {}", msg);
                     return Err(CommandError::from(msg));
                 }
             }
        }
    };
    Ok(total_seconds.max(0.0))
}


// --- Main Cloud Transcription Command ---
#[tauri::command]
pub async fn run_cloud_transcription(
    app_handle: AppHandle,
    media_path: String,
    cloud_model_id: String,
    _language: String,
    num_speakers: usize,
    speaker_names: Vec<String>,
    api_key: String,
    job_id: String,
    cancel_state: State<'_, TranscriptionCancellationState>)
-> Result<TranscriptionResult, CommandError> {
    info!( "[Gemini Transcribe] Start Job '{}': Media='{}', Model='{}', Speakers={}", job_id, media_path, cloud_model_id, num_speakers);
    let cancel_flag = Arc::new(AtomicBool::new(false));
    cancel_state.0.insert(job_id.clone(), Arc::clone(&cancel_flag));

    // FIX: Correctly clone the Arc<DashMap> for the guard
    let _cancel_guard = CancelGuard { job_id: job_id.clone(), state: cancel_state.0.clone() };

    let project_xml_path_buf = get_project_xml_path_from_item(&PathBuf::from(&media_path))?;
    let project_xml_path_str = project_xml_path_buf.to_string_lossy().to_string();
    info!("[Gemini Transcribe][Job '{}'] Found Project XML: {}", job_id, project_xml_path_str);

    let _ = emit_progress(&app_handle, &job_id, 1.0, "Preparing audio...").await;
    let wav_media_path = convert_to_wav_if_needed(&app_handle, &media_path, &job_id, &cancel_flag).await?;
    let _ = emit_progress(&app_handle, &job_id, 5.0, "Reading audio file...").await;
    info!("[Gemini Transcribe][Job '{}'] Using WAV: {}", job_id, wav_media_path.display());

    let audio_content_bytes = match File::open(&wav_media_path).await {
        Ok(mut file) => {
            let mut buffer = Vec::new();
            if let Err(e) = file.read_to_end(&mut buffer).await {
                error!("Failed to read WAV file '{}': {}", wav_media_path.display(), e);
                return Err(CommandError::from(format!("Failed read WAV: {}", e)));
            }
            buffer
        }
        Err(e) => {
            error!("Failed to open WAV file '{}': {}", wav_media_path.display(), e);
            return Err(CommandError::from(format!("Failed open WAV: {}", e)));
        }
    };
    let audio_base64 = BASE64_STANDARD.encode(&audio_content_bytes);
    drop(audio_content_bytes);
    let audio_mime_type = "audio/wav".to_string();
    let _ = emit_progress(&app_handle, &job_id, 10.0, "Preparing API request...").await;
    debug!("[Gemini Transcribe][Job '{}'] Encoded {} bytes of audio data (mime: {})", job_id, audio_base64.len(), audio_mime_type);

    let prompt_text = r#"Generate a transcript of the speech in the audio with speaker diarization. Structure the output strictly as a JSON array where each object represents a segment and has keys "start_time" (hh:mm:ss.ms), "end_time" (hh:mm:ss.ms), "speaker_no" (e.g. "speaker_1", "speaker_2"), and "text" (the transcribed text). For example: [{"start_time": "00:01:26.123", "end_time": "00:01:28.456", "speaker_no": "speaker_1", "text": "Hello world"}, ...]. Output ONLY the JSON array, without any introductory text, explanation, or markdown formatting."#.to_string();

    let request_payload = GeminiRequest {
        contents: vec![GeminiContent {
            parts: vec![
                GeminiPart { text: Some(prompt_text), inline_data: None },
                GeminiPart { text: None, inline_data: Some(GeminiInlineData { mime_type: audio_mime_type, data: audio_base64 }) },
            ]
        }],
    };

    let api_url = format!("https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}", cloud_model_id, api_key);
    info!("[Gemini Transcribe][Job '{}'] Using API Model: {}", job_id, cloud_model_id);

    let _ = emit_progress(&app_handle, &job_id, 15.0, "Sending request to Gemini API...").await;
    let client = ReqwestClient::new();
    let request_builder = client.post(&api_url)
        .header("Content-Type", "application/json")
        .json(&request_payload);
    debug!("[Gemini Transcribe][Job '{}'] POSTing request to {}", job_id, api_url);

    let response_result = tokio::select! {
        biased;
        _ = async {
            loop {
                if cancel_flag.load(Ordering::Relaxed) { break; }
                sleep(Duration::from_millis(100)).await;
            }
        } => {
            warn!("[Gemini Transcribe][Job '{}'] Cancellation detected during API request.", job_id);
            Err(CommandError::from("Cloud transcription cancelled by user."))
        }
        response = request_builder.send() => {
            response.map_err(|e| {
                error!("[Gemini Transcribe][Job '{}'] Failed to send request to Gemini: {}", job_id, e);
                CommandError::from(format!("Failed send request to Gemini API: {}", e))
            })
        }
    };

    let response = match response_result {
        Ok(r) => r,
        Err(e) => {
            return Err(e);
        }
    };

    let _ = emit_progress(&app_handle, &job_id, 60.0, "Received response. Processing...").await;
    let response_status = response.status();
    info!("[Gemini Transcribe][Job '{}'] API Response Status: {}", job_id, response_status);
    let response_text = response.text().await.map_err(|e| CommandError::from(format!("Failed to read Gemini response body: {}", e)))?;
    debug!("[Gemini Transcribe][Job '{}'] API Response Body snippet:\n{}", job_id, response_text.chars().take(1000).collect::<String>());

    // FIX: Remove .await from non-async function call
    let (_, _, _, final_transcript_path) = prepare_output_paths(&media_path, &job_id)?; // No await here
    let raw_json_path = final_transcript_path.with_file_name(format!(
        "{}_gemini_raw.json",
        final_transcript_path.file_stem().unwrap_or_default().to_string_lossy()
    ));
    debug!("[Gemini Transcribe][Job '{}'] Attempting to save raw Gemini JSON response to: {:?}", job_id, raw_json_path);
    if let Err(e) = std_fs::write(&raw_json_path, &response_text) {
        warn!("[Gemini Transcribe][Job '{}'] Failed save raw API output to {:?}: {}", job_id, raw_json_path, e);
    } else {
        info!("[Gemini Transcribe][Job '{}'] Raw Gemini JSON output saved successfully.", job_id);
    }

    if !response_status.is_success() {
        error!("[Gemini Transcribe][Job '{}'] Gemini API returned error status {}. Body: {}", job_id, response_status, response_text);
        return Err(CommandError::from(format!("Gemini API error ({}): {}", response_status, response_text.chars().take(500).collect::<String>())));
    }

    let gemini_response: GeminiResponse = match serde_json::from_str(&response_text) {
        Ok(resp) => resp,
        Err(e) => {
            error!("[Gemini Transcribe][Job '{}'] Failed to parse Gemini JSON response structure: {}. Body was: {}", job_id, e, response_text);
            return Err(CommandError::from(format!("Failed parse Gemini response structure: {}", e)));
        }
    };

    if let Some(feedback) = gemini_response.prompt_feedback {
        if let Some(reason) = feedback.block_reason {
            error!("[Gemini Transcribe][Job '{}'] Gemini blocked the request due to safety settings. Reason: {}", job_id, reason);
            return Err(CommandError::from(format!("Gemini blocked request: {}", reason)));
        }
    }

    let json_text_output_raw = gemini_response.candidates
        .and_then(|c| c.into_iter().next())
        .and_then(|cand| cand.content)
        .and_then(|cont| cont.parts.into_iter().find(|p| p.text.is_some()))
        .and_then(|part| part.text)
        .ok_or_else(|| CommandError::from("Could not extract text content containing JSON transcript from Gemini response."))?;

    let json_text_cleaned = extract_json_array_string(&json_text_output_raw);
    debug!("[Gemini Transcribe][Job '{}'] Attempting to parse cleaned JSON transcript snippet: {}", job_id, json_text_cleaned.chars().take(200).collect::<String>());

    let _ = emit_progress(&app_handle, &job_id, 85.0, "Parsing transcript data...").await;

    let gemini_segments_raw: Vec<GeminiJsonSegment> = match serde_json::from_str(&json_text_cleaned) {
        Ok(parsed) => parsed,
        Err(e) => {
            error!("[Gemini Transcribe][Job '{}'] Failed to parse JSON segment array from Gemini text: {}", job_id, e);
            error!("[Gemini Transcribe][Job '{}'] Cleaned text that failed parsing: {}", job_id, json_text_cleaned);
            return Err(CommandError::from(format!("Gemini returned malformed JSON transcript segments: {}", e)));
        }
    };

    let mut segments: Vec<TranscriptSegment> = Vec::new();
    for (idx, gs) in gemini_segments_raw.iter().enumerate() {
        match (parse_hhmmss_ms_to_seconds(&gs.start_time), parse_hhmmss_ms_to_seconds(&gs.end_time)) {
            (Ok(start), Ok(end)) => {
                if end > start {
                    segments.push(TranscriptSegment {
                        start_time: start,
                        end_time: end,
                        speaker: gs.speaker_no.clone(),
                        text: gs.text.clone(),
                    });
                } else {
                    warn!("[Gemini Parse][Job '{}'] Skipping segment {} due to end time ({}) <= start time ({}): Text='{}...'", job_id, idx, gs.end_time, gs.start_time, gs.text.chars().take(30).collect::<String>());
                }
            }
            (Err(e_start), _) => { error!("[Gemini Parse][Job '{}'] Failed parsing start time '{}' for segment {}: {}", job_id, gs.start_time, idx, e_start.message); }
            (_, Err(e_end)) => { error!("[Gemini Parse][Job '{}'] Failed parsing end time '{}' for segment {}: {}", job_id, gs.end_time, idx, e_end.message); }
        }
    }
    info!("[Gemini Transcribe][Job '{}'] Successfully parsed {} segments from Gemini response.", job_id, segments.len());

    let _ = emit_progress(&app_handle, &job_id, 90.0, "Mapping speaker names...").await;
    map_speaker_ids_to_names(&mut segments, &speaker_names);

    debug!("[Gemini Transcribe][Job '{}'] Saving final processed transcript JSON to: {:?}", job_id, final_transcript_path);
    save_transcript_json(
        project_xml_path_str,
        final_transcript_path.to_string_lossy().to_string(),
        segments.clone(),
    ).await?;
    info!("[Gemini Transcribe][Job '{}'] Final processed transcript saved.", job_id);

    // Convert returned segments into Lexical JSON strings for UI consumption
    let lexical_segments: Vec<TranscriptSegment> = segments.iter().cloned().map(|mut seg| {
        let doc = generate_lexical_doc(&seg.text);
        if let Ok(json_str) = serde_json::to_string(&doc) {
            seg.text = json_str;
        }
        seg
    }).collect();

    info!("[Gemini Transcribe][Job '{}'] Cloud transcription process complete.", job_id);
    let _ = emit_progress(&app_handle, &job_id, 100.0, "Transcription complete.").await;

    Ok(TranscriptionResult {
        segments: lexical_segments,
        transcript_file_path: final_transcript_path.to_string_lossy().to_string(),
    })
}

// Helper function to extract JSON array string
fn extract_json_array_string(raw_text: &str) -> String {
    let trimmed = raw_text
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    if let (Some(start_index), Some(end_index)) = (trimmed.find('['), trimmed.rfind(']')) {
        if start_index < end_index {
             debug!("[Gemini Parse] Extracted JSON array string successfully.");
            return trimmed[start_index..=end_index].to_string();
        }
    }
    warn!("[Gemini Parse] Could not reliably extract JSON array string using '[' and ']'. Returning trimmed text. Raw start: '{}'", raw_text.chars().take(50).collect::<String>());
    trimmed.to_string()
}

// --- Cloud Cancellation Command ---
#[tauri::command]
pub async fn cancel_cloud_transcription(
    job_id: String,
    cancel_state: State<'_, TranscriptionCancellationState>)
-> Result<(), CommandError> {
    info!("[Gemini Transcribe] Received cancellation request for job: {}", job_id);
    if let Some(flag_entry) = cancel_state.0.get(&job_id) {
        let cancel_flag = flag_entry.value();
        match cancel_flag.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst) {
            Ok(_) => { info!("[Gemini Transcribe] Cancellation flag successfully SET for job: {}", job_id); }
            Err(_) => { info!("[Gemini Transcribe] Cancellation flag was already SET for job: {}", job_id); }
        }
    } else {
        warn!("[Gemini Transcribe] Cancellation request for unknown or already completed job ID: {}", job_id);
    }
    Ok(())
}


// --- RAII Guard for Cancellation State Cleanup ---
struct CancelGuard {
    job_id: String,
    state: Arc<dashmap::DashMap<String, Arc<AtomicBool>>>,
}

impl Drop for CancelGuard {
    fn drop(&mut self) {
        if self.state.remove(&self.job_id).is_some() {
            debug!("[CancelGuard] Removed cancel flag for job '{}' on drop.", self.job_id);
        } else {
             warn!("[CancelGuard] Attempted to remove flag for job '{}' on drop, but it was already gone.", self.job_id);
        }
    }
}