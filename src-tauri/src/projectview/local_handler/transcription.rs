// src-tauri/src/projectview/local_handler/transcription.rs

use crate::projectview::shared_types::{TranscriptSegment, ProgressPayload, TranscriptionResult};
use crate::projectview::shared_utils::{get_project_xml_path_from_item};
use crate::projectview::transcription_commands::{ // Import transcription commands
    prepare_output_paths, save_transcript_json, map_speaker_ids_to_names, 
    create_lexical_paragraph_json_value, // CHANGED: Renamed from generate_lexical_doc
    create_lexical_table_from_segments // ADDED: New function to import
};
use serde_json;
use crate::welcome::config::{get_default_download_location, read_config, CommandError};
use crate::TranscriptionCancellationState;

use log::{debug, error, info, warn};
use serde::{Deserialize}; 
use std::{
    cmp::Ordering as CmpOrdering,
    collections::HashMap,
    fs::{self, File},
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_shell::{process::CommandEvent, ShellExt};
use tokio::time::sleep;

// Structs specific to parsing whisper output
#[derive(Deserialize, Debug)] struct WhisperJsonOutput { transcription: Option<Vec<WhisperJsonSegment>> }
#[derive(Deserialize, Debug)] struct WhisperJsonSegment { timestamps: WhisperJsonTimestamps, text: String }
#[derive(Deserialize, Debug)] struct WhisperJsonTimestamps { from: String, to: String }

// Struct specific to parsing RTTM output
#[derive(Debug, Clone)] struct RttmRecord { start_time: f64, duration: f64, speaker_id: String }


// --- Main Transcription Command ---
#[tauri::command]
pub async fn run_transcription(
    app_handle: AppHandle,
    media_path: String,
    model_name: String,
    language: String,
    num_speakers: usize,
    speaker_names: Vec<String>,
    job_id: String,
    cancel_state: State<'_, TranscriptionCancellationState>)
-> Result<TranscriptionResult, CommandError> {
    info!( "[Transcription] Start Job '{}': Media='{}', Model='{}', Lang='{}', Speakers={}", job_id, media_path, model_name, language, num_speakers);
    let cancel_flag = Arc::new(AtomicBool::new(false));
    cancel_state.0.insert(job_id.clone(), Arc::clone(&cancel_flag));

    let _cancel_guard = CancelGuard { job_id: job_id.clone(), state: cancel_state.0.clone() };

    let project_xml_path_buf = get_project_xml_path_from_item(&PathBuf::from(&media_path))?;
    let project_xml_path_str = project_xml_path_buf.to_string_lossy().to_string();
    info!("[Transcription][Job '{}'] Found Project XML: {}", job_id, project_xml_path_str);

    let _ = emit_progress(&app_handle, &job_id, 1.0, "Preparing audio...").await;
    let wav_media_path = convert_to_wav_if_needed(&app_handle, &media_path, &job_id, &cancel_flag).await?;
    let _ = emit_progress(&app_handle, &job_id, 4.0, "Audio ready.").await;
    info!("[Transcription][Job '{}'] Using WAV path: {}", job_id, wav_media_path.display());

    let whisper_model_path_str = resolve_whisper_model_path(&model_name, &job_id).await?;
    debug!("[Transcription][Job '{}'] Whisper model path: '{}'", job_id, whisper_model_path_str);

    let (output_path_base_str, expected_whisper_output_path, expected_rttm_path, final_transcript_path) =
        prepare_output_paths(&wav_media_path.to_string_lossy(), &job_id)?; 
    debug!("[Transcription][Job '{}'] Paths - Base:'{}', Whisper:'{}', RTTM:'{}', Final:'{}'", job_id, output_path_base_str, expected_whisper_output_path.display(), expected_rttm_path.display(), final_transcript_path.display());

    let _ = emit_progress(&app_handle, &job_id, 5.0, "Running transcription...").await;
    let whisper_output_path = run_whisper_cpp_sidecar(
        &app_handle,
        &wav_media_path.to_string_lossy(),
        &whisper_model_path_str,
        &language,
        &job_id,
        &cancel_flag,
        &output_path_base_str, // This is the base name for whisper output files (e.g., /path/to/transcripts/media_stem)
        &expected_whisper_output_path, // This is the expected path of the .json file (e.g., /path/to/transcripts/media_stem.json)
    ).await?;

    let _ = emit_progress(&app_handle, &job_id, 45.0, "Parsing results...").await;
    // These are plain text segments after parsing whisper's raw output
    let mut whisper_segments_plain = parse_whisper_json(&whisper_output_path)?; 
    debug!("[Transcription][Job '{}'] Parsed {} plain text segments.", job_id, whisper_segments_plain.len());

    let rttm_records: Option<Vec<RttmRecord>> = if num_speakers > 0 {
        let _ = emit_progress(&app_handle, &job_id, 50.0, "Running diarization...").await;
        match run_diarize_cli_sidecar(
            &app_handle,
            "diarize-cli",
            &wav_media_path.to_string_lossy(),
            num_speakers,
            &expected_rttm_path, // This is the expected path for the .rttm file
            &job_id,
            &cancel_flag
        ).await {
            Ok(rttm_path) => {
                match parse_rttm_file(&rttm_path) {
                    Ok(records) => {
                        debug!("[Transcription][Job '{}'] Diarization success, {} turns.", job_id, records.len());
                        let _ = emit_progress(&app_handle, &job_id, 85.0, "Merging results...").await;
                        Some(records)
                    }
                    Err(e) => {
                        error!("[Transcription][Job '{}'] Failed parse RTTM '{}': {}", job_id, rttm_path.display(), e.message);
                        let _ = emit_progress(&app_handle, &job_id, 85.0, "RTTM parse failed.").await;
                        None
                    }
                }
            },
            Err(e) => {
                if e.message.contains("cancelled") || e.message.contains("canceled") {
                    info!("[Transcription][Job '{}'] Diarization explicitly cancelled.", job_id);
                     return Err(CommandError::from("Diarization cancelled."));
                 } else {
                    error!("[Transcription][Job '{}'] Diarization failed: {}.", job_id, e.message);
                    warn!("Ensure diarization CLI (diarize-cli) is installed and accessible (e.g., via pipx or venv).");
                    let _ = emit_progress(&app_handle, &job_id, 85.0, "Diarization failed.").await;
                    None
                }
            }
        }
    } else {
        info!("[Transcription][Job '{}'] Skipping diarization (num_speakers=0).", job_id);
        let _ = emit_progress(&app_handle, &job_id, 85.0, "Skipping diarization.").await;
        None
    };

    if let Some(rttm_data) = &rttm_records {
        if !rttm_data.is_empty() {
            debug!("[Transcription][Job '{}'] Merging diarization results...", job_id);
            merge_diarization_results(&mut whisper_segments_plain, rttm_data); 
            let _ = emit_progress(&app_handle, &job_id, 90.0, "Mapping speaker names...").await;
            map_speaker_ids_to_names(&mut whisper_segments_plain, &speaker_names);
        } else {
            warn!("[Transcription][Job '{}'] Diarization ran but resulted in 0 RTTM records.", job_id);
            let _ = emit_progress(&app_handle, &job_id, 90.0, "No speaker segments found.").await;
        }
    } else {
        debug!("[Transcription][Job '{}'] No RTTM data to merge.", job_id);
    }

    let _ = emit_progress(&app_handle, &job_id, 95.0, "Saving final transcript...").await;
    
    // --- MODIFICATION: Generate Lexical Table JSON ---
    let lexical_table_json_value = create_lexical_table_from_segments(&whisper_segments_plain);
    let lexical_table_json_string = serde_json::to_string_pretty(&lexical_table_json_value)
        .map_err(|e| CommandError::from(format!("Failed to serialize Lexical Table JSON: {}", e)))?;
    // --- END MODIFICATION ---

    debug!("[Transcription][Job '{}'] Saving final Lexical Table JSON to: {:?}", job_id, final_transcript_path);
    save_transcript_json(
        project_xml_path_str,
        final_transcript_path.to_string_lossy().to_string(),
        lexical_table_json_string, // MODIFIED: Pass the string representation of the Lexical Table
    ).await?;
    info!("[Transcription][Job '{}'] Final transcript saved.", job_id);

    // --- MODIFICATION: Prepare segments for frontend (text field will contain Lexical JSON for the cell's content) ---
    // The `whisper_segments_plain` still holds the segments with plain text, which is what we want
    // to use as the basis for the `text` field of each segment in the result.
    let segments_for_frontend_result: Vec<TranscriptSegment> = whisper_segments_plain.iter().cloned().map(|seg_plain| {
        // For each plain text segment, create the Lexical JSON for its *text cell content*
        let cell_content_lexical_value = create_lexical_paragraph_json_value(&seg_plain.text);
        let cell_content_lexical_string = serde_json::to_string(&cell_content_lexical_value)
            .unwrap_or_else(|_| serde_json::to_string(&create_lexical_paragraph_json_value("")).unwrap()); // Fallback to empty paragraph

        TranscriptSegment {
            start_time: seg_plain.start_time,
            end_time: seg_plain.end_time,
            speaker: seg_plain.speaker.clone(),
            text: cell_content_lexical_string, // This `text` is the Lexical JSON for the cell
        }
    }).collect();
    // --- END MODIFICATION ---

    info!("[Transcription][Job '{}'] Process complete.", job_id);
    let _ = emit_progress(&app_handle, &job_id, 100.0, "Transcription complete.").await;

    Ok(TranscriptionResult {
        segments: segments_for_frontend_result, // MODIFIED
        transcript_file_path: final_transcript_path.to_string_lossy().to_string(),
    })
}

// --- Helper: Convert to WAV using FFmpeg ---
pub(crate) async fn convert_to_wav_if_needed(
    app_handle: &AppHandle,
    input_path_str: &str,
    job_id: &str,
    cancel_flag: &Arc<AtomicBool>)
-> Result<PathBuf, CommandError> {
    info!("[FFmpeg][{}] Checking audio file: {}", job_id, input_path_str);
    let input_path = PathBuf::from(input_path_str);
    let extension = input_path.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();

    if extension == "wav" {
        info!("[FFmpeg][{}] Input is already WAV. Skipping conversion.", job_id);
        return Ok(input_path);
    }

    let output_wav_path = input_path.with_extension("wav");
    info!("[FFmpeg][{}] Target WAV path: {}", job_id, output_wav_path.display());

    if output_wav_path.exists() {
        match output_wav_path.metadata() {
            Ok(m) if m.len() > 0 => {
                info!("[FFmpeg][{}] Target WAV file already exists and is not empty. Reusing.", job_id);
                return Ok(output_wav_path);
            },
            _ => {
                warn!("[FFmpeg][{}] Target WAV file exists but is empty or metadata error. Overwriting.", job_id);
            }
        }
    }

    info!("[FFmpeg][{}] Starting FFmpeg conversion...", job_id);
    let _ = emit_progress(app_handle, job_id, 2.0, "Converting audio to WAV...").await;

    let args: Vec<String> = vec![
        "-i".into(), input_path_str.to_string(),
        "-vn".into(),
        "-acodec".into(), "pcm_s16le".into(),
        "-ar".into(), "16000".into(),
        "-ac".into(), "1".into(),
        "-y".into(),
        output_wav_path.to_string_lossy().to_string(),
    ];
    debug!("[FFmpeg][{}] Command arguments: {:?}", job_id, args);

    let shell_scope = app_handle.shell();
    let (mut rx, child) = shell_scope
        .sidecar("ffmpeg")?
        .args(args)
        .spawn()?;
    debug!("[FFmpeg][{}] Spawned FFmpeg process (PID: {:?})", job_id, child.pid());

    let mut ffmpeg_stderr: Vec<String> = Vec::new();
    let mut ffmpeg_exit_code: Option<i32> = None;
    let mut ffmpeg_error: Option<String> = None;

    loop {
        if cancel_flag.load(Ordering::Relaxed) {
            warn!("[FFmpeg][{}] Cancellation requested. Killing FFmpeg process...", job_id);
            let _ = child.kill();
            if output_wav_path.exists() { let _ = fs::remove_file(&output_wav_path); }
            return Err(CommandError::from("Audio conversion cancelled."));
        }

        tokio::select! {
            biased;
            maybe_event = rx.recv() => {
                match maybe_event {
                    Some(event) => match event {
                        CommandEvent::Stdout(line) => { debug!("[FFmpeg][stdout][{}] {}", job_id, String::from_utf8_lossy(&line).trim_end()); },
                        CommandEvent::Stderr(line) => { let l = String::from_utf8_lossy(&line).to_string(); debug!("[FFmpeg][stderr][{}] {}", job_id, l.trim_end()); ffmpeg_stderr.push(l); }
                        CommandEvent::Error(msg) => { error!("[FFmpeg][error][{}] {}", job_id, msg); ffmpeg_error = Some(msg); break; }
                        CommandEvent::Terminated(payload) => { info!("[FFmpeg][term][{}] Process terminated. Code: {:?}, Signal: {:?}", job_id, payload.code, payload.signal); ffmpeg_exit_code = payload.code; if payload.signal.is_some() && ffmpeg_exit_code.is_none() { ffmpeg_exit_code = Some(-1); } break; }
                        _ => {}
                    },
                    None => {
                        if ffmpeg_exit_code.is_none() && ffmpeg_error.is_none() {
                            warn!("[FFmpeg][{}] Event channel closed unexpectedly before termination signal.", job_id);
                            ffmpeg_exit_code = Some(-1);
                        }
                        break;
                    }
                }
            }
            _ = sleep(Duration::from_millis(50)) => {
                 continue;
            }
        }
    }

    let stderr_output = ffmpeg_stderr.join("\n");
    if ffmpeg_error.is_some() || ffmpeg_exit_code != Some(0) {
        error!("[FFmpeg][{}] FFmpeg process failed. Code: {:?}, Error: {:?}\nStderr:\n{}", job_id, ffmpeg_exit_code, ffmpeg_error, stderr_output);
        if output_wav_path.exists() { let _ = fs::remove_file(&output_wav_path); }
        return Err(CommandError::from(format!("FFmpeg conversion failed. Code: {:?}. Error: {}", ffmpeg_exit_code, ffmpeg_error.unwrap_or_default())));
    }

    if !output_wav_path.exists() {
        error!("[FFmpeg][{}] FFmpeg reported success, but output file is missing: {}", job_id, output_wav_path.display());
        return Err(CommandError::from(format!("FFmpeg conversion failed: output file missing ({})", output_wav_path.display())));
    }
    match output_wav_path.metadata() {
        Ok(m) if m.len() == 0 => {
            error!("[FFmpeg][{}] FFmpeg reported success, but output file is empty: {}", job_id, output_wav_path.display());
            let _ = fs::remove_file(&output_wav_path);
            return Err(CommandError::from(format!("FFmpeg conversion failed: output file is empty ({})", output_wav_path.display())));
        },
        Err(e) => {
            error!("[FFmpeg][{}] FFmpeg reported success, but failed to get metadata for {}: {}", job_id, output_wav_path.display(), e);
            let _ = fs::remove_file(&output_wav_path);
            return Err(CommandError::from(format!("FFmpeg conversion failed: output metadata error ({})", e)));
        },
        Ok(_) => {}
    }

    info!("[FFmpeg][{}] Successfully converted '{}' to WAV: {}", job_id, input_path_str, output_wav_path.display());
    Ok(output_wav_path)
}

// --- Helper: Resolve Whisper Model Path ---
async fn resolve_whisper_model_path( model_name: &str, job_id: &str) -> Result<String, CommandError> {
    let config = read_config()?;
    let base_model_dir_str = if !config.download_location.trim().is_empty() {
        config.download_location
    } else {
        get_default_download_location()?
    };
    let model_dir_path = PathBuf::from(&base_model_dir_str).join(model_name);

    if !model_dir_path.exists() || !model_dir_path.is_dir() {
        let e = format!("Model directory not found: '{}'. Please download the model first.", model_dir_path.display());
        error!("[Transcription][{}] Error resolving model path: {}", job_id, e);
        return Err(CommandError::from(e));
    }
    let model_file_path = find_model_file(&model_dir_path)?;
    Ok(model_file_path.to_string_lossy().to_string())
}

// --- Helper: Run whisper-cpp Sidecar ---
async fn run_whisper_cpp_sidecar(
    app_handle: &AppHandle,
    media_path: &str,
    whisper_model_path_str: &str,
    language: &str,
    job_id: &str,
    cancel_flag: &Arc<AtomicBool>,
    output_path_base_str: &str, // Base for whisper's -of flag (e.g., .../transcripts/media_stem_temp_jobid)
    expected_output_path: &Path // Full path to the .json file whisper should create (e.g., .../media_stem_temp_jobid.whisper.json)
) -> Result<PathBuf, CommandError> {
    let sidecar_name = "whisper-cpp";
    let lang_arg = if language.trim().is_empty() || language == "auto" { "auto" } else { language.trim() };
    debug!("[Transcription][{}] Using Whisper language: '{}'", job_id, lang_arg);

    let args: Vec<String> = vec![
        "-m".into(), whisper_model_path_str.to_string(),
        "-f".into(), media_path.to_string(),
        "-l".into(), lang_arg.to_string(),
        "-oj".into(), // Output JSON
        "-of".into(), output_path_base_str.to_string(), // Output file base name
    ];
    debug!("[Transcription][{}] Running sidecar '{}' with args: {:?}", job_id, sidecar_name, args);

    let shell_scope = app_handle.shell();
    let (mut rx, child) = shell_scope.sidecar(sidecar_name)?.args(args).spawn()
     .map_err(|e| {
         error!("Failed to spawn whisper-cpp: {}. Check tauri.conf.json, binary paths, and permissions.", e);
         CommandError::from(format!("Failed to execute whisper-cpp sidecar: {}. Ensure it's bundled and executable.", e))
     })?;
    info!("[Transcription][{}] Spawned sidecar '{}' (PID: {:?})", job_id, sidecar_name, child.pid());

    let mut stderr_lines = Vec::new();
    let mut _stdout_lines = Vec::new();
    let mut process_error: Option<String> = None;
    let mut exit_code: Option<i32> = None;

    loop {
        if cancel_flag.load(Ordering::Relaxed) {
            warn!("[Transcription][{}] Cancellation requested during '{}'. Killing process...", job_id, sidecar_name);
            let _ = child.kill();
            if expected_output_path.exists() { let _ = fs::remove_file(expected_output_path); }
            return Err(CommandError::from("Whisper-cpp process cancelled."));
        }

        tokio::select! {
            biased;
            maybe_event = rx.recv() => {
                match maybe_event {
                    Some(event) => match event {
                        CommandEvent::Stdout(line) => { _stdout_lines.push(String::from_utf8_lossy(&line).to_string()); },
                        CommandEvent::Stderr(line) => { let l = String::from_utf8_lossy(&line).to_string(); debug!("[{}][stderr][{}] {}", sidecar_name, job_id, l.trim_end()); stderr_lines.push(l); },
                        CommandEvent::Error(msg) => { error!("[{}][error][{}] {}", sidecar_name, job_id, msg); process_error = Some(msg); break; },
                        CommandEvent::Terminated(payload) => { info!("[{}][term][{}] Process terminated. Code: {:?}, Signal: {:?}", sidecar_name, job_id, payload.code, payload.signal); exit_code = payload.code; if payload.signal.is_some() && exit_code.is_none() { exit_code = Some(-1); } break; },
                        _ => {}
                    },
                    None => {
                        if exit_code.is_none() && process_error.is_none() { warn!("[{}][{}] Event channel closed unexpectedly.", sidecar_name, job_id); exit_code = Some(-1); }
                        break;
                    }
                }
            }
            _ = sleep(Duration::from_millis(100)) => {
                continue;
            }
        }
    }

    let final_stderr = stderr_lines.join("\n");
    info!("[Transcription][{}] Sidecar '{}' finished. Code: {:?}, Error: {:?}.", job_id, sidecar_name, exit_code, process_error);

    if !final_stderr.is_empty() {
        if process_error.is_some() || exit_code != Some(0) {
            error!("[Transcription][{}] '{}' Stderr output on failure:\n{}", job_id, sidecar_name, final_stderr);
        } else {
            debug!("[Transcription][{}] '{}' Stderr output on success:\n{}", job_id, sidecar_name, final_stderr);
        }
    }

    if process_error.is_some() || exit_code != Some(0) {
        let ec_str = exit_code.map_or("N/A".to_string(), |c| c.to_string());
        let error_message = format!("Sidecar '{}' failed. Exit Code: {}. Error: {}. Stderr: {}",
            sidecar_name,
            ec_str,
            process_error.unwrap_or_default(),
            final_stderr.chars().take(500).collect::<String>()
        );
        error!("[Transcription][{}] {}", job_id, error_message);
        if expected_output_path.exists() { let _ = fs::remove_file(expected_output_path); }
        return Err(CommandError::from(error_message));
    }

    let mut attempts = 0;
    while !expected_output_path.exists() && attempts < 5 {
        attempts += 1;
        warn!("[Transcription][{}] Output JSON '{:?}' not found yet, waiting {}ms (attempt {}/5)...", job_id, expected_output_path, 300, attempts);
        sleep(Duration::from_millis(300)).await;
        if cancel_flag.load(Ordering::Relaxed) {
            if expected_output_path.exists() { let _ = fs::remove_file(expected_output_path); }
            return Err(CommandError::from("Cancelled while waiting for whisper output file."));
        }
    }

    if !expected_output_path.exists() {
        return Err(CommandError::from(format!("Sidecar '{}' completed successfully, but output file is missing: {:?}", sidecar_name, expected_output_path)));
    }
    match expected_output_path.metadata() {
        Ok(m) if m.len() == 0 => {
            warn!("[Transcription][{}] Output JSON file exists but is empty: {:?}", job_id, expected_output_path);
            let _ = fs::remove_file(expected_output_path);
            return Err(CommandError::from(format!("Sidecar '{}' completed, but output file was empty: {:?}", sidecar_name, expected_output_path)));
        },
        Err(e) => {
            error!("[Transcription][{}] Failed to get metadata for output file {}: {}", job_id, expected_output_path.display(), e);
            let _ = fs::remove_file(expected_output_path);
            return Err(CommandError::from(format!("Output file validation error: {}", e)));
        },
        Ok(_) => {}
    }

    info!("[Transcription][{}] Output JSON created successfully by '{}': {:?}", job_id, sidecar_name, expected_output_path);
    Ok(expected_output_path.to_path_buf())
}


// --- Helper: Parse Whisper JSON ---
fn parse_whisper_json(json_path: &Path) -> Result<Vec<TranscriptSegment>, CommandError> {
    debug!("[JSON Parse] Reading whisper output: {:?}", json_path);
    let file = File::open(json_path)?;
    let reader = BufReader::new(file);
    let output: WhisperJsonOutput = serde_json::from_reader(reader)
        .map_err(|e| CommandError::from(format!("Failed to parse whisper JSON from '{}': {}", json_path.display(), e)))?;

    let mut segments = Vec::new();
    if let Some(transcription) = output.transcription {
        for (idx, w_seg) in transcription.iter().enumerate() {
             let start_time = parse_whisper_timestamp(&w_seg.timestamps.from)
                .map_err(|e_msg| CommandError::from(format!("Segment {}: Invalid start time '{}': {}", idx, w_seg.timestamps.from, e_msg)))?;
            let end_time = parse_whisper_timestamp(&w_seg.timestamps.to)
                 .map_err(|e_msg| CommandError::from(format!("Segment {}: Invalid end time '{}': {}", idx, w_seg.timestamps.to, e_msg)))?;

            if end_time < start_time {
                warn!("[JSON Parse] Skipping segment {} due to end time ({}) < start time ({}): '{}'", idx, end_time, start_time, w_seg.text.trim());
                continue;
            }
            segments.push(TranscriptSegment {
                start_time,
                end_time,
                speaker: "Unknown".to_string(), // Default speaker
                text: w_seg.text.trim().to_string(),
            });
        }
    } else {
        warn!("[JSON Parse] No 'transcription' array found in whisper JSON file: {:?}", json_path);
    }
    info!("[JSON Parse] Parsed {} segments from {}", segments.len(), json_path.display());
    Ok(segments)
}

// --- Helper: Parse Whisper Timestamp (hh:mm:ss,ms) ---
fn parse_whisper_timestamp(timestamp_str: &str) -> Result<f64, String> {
    let parts: Vec<&str> = timestamp_str.split(':').collect();
    if parts.len() != 3 {
        return Err(format!("Invalid time format (expected hh:mm:ss,ms): '{}'", timestamp_str));
    }
    let hours: u64 = parts[0].parse().map_err(|e| format!("Invalid hours '{}': {}", parts[0], e))?;
    let minutes: u64 = parts[1].parse().map_err(|e| format!("Invalid minutes '{}': {}", parts[1], e))?;

    let sec_ms_parts: Vec<&str> = parts[2].split(',').collect();
    if sec_ms_parts.len() != 2 {
        let sec_ms_parts_dot: Vec<&str> = parts[2].split('.').collect();
         if sec_ms_parts_dot.len() != 2 {
             return Err(format!("Invalid seconds/milliseconds format (expected ss,ms or ss.ms): '{}'", parts[2]));
         }
         let seconds: u64 = sec_ms_parts_dot[0].parse().map_err(|e| format!("Invalid seconds '{}': {}", sec_ms_parts_dot[0], e))?;
         let millis: u32 = sec_ms_parts_dot[1].parse().map_err(|e| format!("Invalid milliseconds '{}': {}", sec_ms_parts_dot[1], e))?;
         Ok((hours * 3600 + minutes * 60 + seconds) as f64 + (millis as f64 / 1000.0))

    } else {
        let seconds: u64 = sec_ms_parts[0].parse().map_err(|e| format!("Invalid seconds '{}': {}", sec_ms_parts[0], e))?;
        let millis: u32 = sec_ms_parts[1].parse().map_err(|e| format!("Invalid milliseconds '{}': {}", sec_ms_parts[1], e))?;
        Ok((hours * 3600 + minutes * 60 + seconds) as f64 + (millis as f64 / 1000.0))
    }
}

// --- Helper: Run Diarization CLI Sidecar ---
async fn run_diarize_cli_sidecar(
    app_handle: &AppHandle,
    sidecar_name: &str,
    media_path: &str,
    num_speakers: usize,
    output_rttm_path: &Path,
    job_id: &str,
    cancel_flag: &Arc<AtomicBool>
) -> Result<PathBuf, CommandError> {
    info!("[DiarizeCLI][{}] Starting diarization for: {}", job_id, media_path);
    if let Some(parent_dir) = output_rttm_path.parent() {
        fs::create_dir_all(parent_dir)?;
    } else {
        return Err(CommandError::from(format!("Could not get parent directory for RTTM output: {}", output_rttm_path.display())));
    }

    let mut args = vec![
        "--audio".into(), media_path.to_string(),
        "--output".into(), output_rttm_path.to_string_lossy().to_string(),
    ];
    if num_speakers > 0 {
        args.push("--num_speakers".into()); args.push(num_speakers.to_string());
        args.push("--min_speakers".into()); args.push(1.to_string());
        args.push("--max_speakers".into()); args.push(num_speakers.max(1).to_string());
        debug!("[DiarizeCLI][{}] Using speaker count hint: [1, {}]", job_id, num_speakers.max(1));
    } else {
        warn!("[DiarizeCLI][{}] num_speakers=0 provided, running diarization without speaker count hints.", job_id);
    }

    debug!("[DiarizeCLI][{}] Running sidecar '{}' with args: {:?}", job_id, sidecar_name, args);
    let shell_scope = app_handle.shell();
    let (mut rx, child) = shell_scope.sidecar(sidecar_name)?.args(args).spawn()
      .map_err(|e| {
          error!("Failed to spawn {}: {}. Ensure Python environment with pyannote.audio and diarize-cli is set up (e.g., via pipx or venv) and accessible.", sidecar_name, e);
          CommandError::from(format!("Failed to execute {} sidecar: {}. Check Python environment.", sidecar_name, e))
      })?;
    debug!("[DiarizeCLI][{}] Spawned '{}' process (PID: {:?})", job_id, sidecar_name, child.pid());

    let mut stderr_lines = Vec::new();
    let mut stdout_lines = Vec::new();
    let mut process_error: Option<String> = None;
    let mut exit_code: Option<i32> = None;

    loop {
        if cancel_flag.load(Ordering::Relaxed) {
            warn!("[DiarizeCLI][{}] Cancellation requested during '{}'. Killing process...", job_id, sidecar_name);
            let _ = child.kill();
            if output_rttm_path.exists() { let _ = fs::remove_file(output_rttm_path); }
            return Err(CommandError::from("Diarization process cancelled."));
        }

        tokio::select! {
            biased;
            maybe_event = rx.recv() => {
                match maybe_event {
                    Some(event) => match event {
                        CommandEvent::Stdout(line) => { let l = String::from_utf8_lossy(&line); debug!("[{}][stdout][{}] {}", sidecar_name, job_id, l.trim_end()); stdout_lines.push(l.to_string()); },
                        CommandEvent::Stderr(line) => { let l = String::from_utf8_lossy(&line); debug!("[{}][stderr][{}] {}", sidecar_name, job_id, l.trim_end()); stderr_lines.push(l.to_string()); }
                        CommandEvent::Error(msg) => { error!("[{}][error][{}] {}", sidecar_name, job_id, msg); process_error = Some(msg); break; }
                        CommandEvent::Terminated(payload) => { info!("[{}][term][{}] Process terminated. Code: {:?}, Signal: {:?}", sidecar_name, job_id, payload.code, payload.signal); exit_code = payload.code; if payload.signal.is_some() && exit_code.is_none() { exit_code = Some(-1); } break; }
                        _ => {}
                    },
                    None => { if exit_code.is_none() && process_error.is_none() { warn!("[{}][{}] Event channel closed unexpectedly.", sidecar_name, job_id); exit_code = Some(-1); } break; }
                }
            }
            _ = sleep(Duration::from_millis(100)) => { continue; }
        }
    }

    let final_stdout = stdout_lines.join("\n");
    let final_stderr = stderr_lines.join("\n");
    info!("[DiarizeCLI][{}] Sidecar '{}' finished. Code: {:?}, Error: {:?}.", job_id, sidecar_name, exit_code, process_error);

    if !final_stdout.is_empty() { debug!("[DiarizeCLI][{}] '{}' Stdout:\n{}", job_id, sidecar_name, final_stdout); }
    if !final_stderr.is_empty() {
        if process_error.is_some() || exit_code != Some(0) { error!("[DiarizeCLI][{}] '{}' Stderr:\n{}", job_id, sidecar_name, final_stderr); }
        else { debug!("[DiarizeCLI][{}] '{}' Stderr:\n{}", job_id, sidecar_name, final_stderr); }
    }

    if process_error.is_some() || exit_code != Some(0) {
        let ec_str = exit_code.map_or("N/A".to_string(), |c| c.to_string());
        let error_message = format!("Sidecar '{}' failed. Exit Code: {}. Error: {}. Stderr: {}",
            sidecar_name, ec_str, process_error.unwrap_or_default(), final_stderr.chars().take(500).collect::<String>());
        error!("[DiarizeCLI][{}] Error: {}", job_id, error_message);
        if output_rttm_path.exists() { let _ = fs::remove_file(output_rttm_path); }
        return Err(CommandError::from(error_message));
    }

    let mut attempts = 0;
    while !output_rttm_path.exists() && attempts < 5 {
        attempts += 1;
        warn!("[DiarizeCLI][{}] Output RTTM '{:?}' not found yet, waiting {}ms (attempt {}/5)...", job_id, output_rttm_path, 200, attempts);
        sleep(Duration::from_millis(200)).await;
        if cancel_flag.load(Ordering::Relaxed) {
            if output_rttm_path.exists() { let _ = fs::remove_file(output_rttm_path); }
            return Err(CommandError::from("Cancelled while waiting for diarization output file."));
        }
    }

    if !output_rttm_path.exists() {
        return Err(CommandError::from(format!("Sidecar '{}' completed successfully, but RTTM output file is missing: {:?}", sidecar_name, output_rttm_path)));
    }
    match output_rttm_path.metadata() {
        Ok(m) if m.len() == 0 => {
            warn!("[DiarizeCLI][{}] Output RTTM file exists but is empty: {:?}", job_id, output_rttm_path);
            // Don't error out here, an empty RTTM might be valid if no speech turns are found.
            // parse_rttm_file will return an empty Vec in this case.
        },
        Err(e) => {
            error!("[DiarizeCLI][{}] Failed to get metadata for RTTM output file {}: {}", job_id, output_rttm_path.display(), e);
            let _ = fs::remove_file(output_rttm_path);
            return Err(CommandError::from(format!("RTTM output file validation error: {}", e)));
        },
        Ok(_) => {}
    }

    info!("[DiarizeCLI][{}] RTTM file created successfully by '{}': {:?}", job_id, sidecar_name, output_rttm_path);
    Ok(output_rttm_path.to_path_buf())
}

// --- Helper: Parse RTTM File ---
fn parse_rttm_file(rttm_path: &Path) -> Result<Vec<RttmRecord>, CommandError> {
    debug!("[RTTM Parse] Reading RTTM file: {:?}", rttm_path);
    let file = File::open(rttm_path)?;
    let reader = BufReader::new(file);
    let mut records = Vec::new();

    for (line_num, line_result) in reader.lines().enumerate() {
        let line_number = line_num + 1;
        let line = line_result?;

        if line.trim().is_empty() || line.starts_with(';') {
            continue;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 8 {
            warn!("[RTTM Parse] Skipping malformed line {} ({} fields): '{}'", line_number, parts.len(), line);
            continue;
        }

        if parts.first().map_or(false, |&t| t != "SPEAKER") {
            debug!("[RTTM Parse] Skipping non-SPEAKER line {}: '{}'", line_number, line);
            continue;
        }

        let start_str = parts[3];
        let duration_str = parts[4];
        let speaker_id = parts[7].to_string();

        let start_time: f64 = match start_str.parse() {
            Ok(t) if t >= 0.0 => t,
            _ => {
                warn!("[RTTM Parse] Skipping line {} due to invalid start time '{}'", line_number, start_str);
                continue;
            }
        };

        let duration: f64 = match duration_str.parse() {
            Ok(d) if d > 0.0 => d,
            _ => {
                warn!("[RTTM Parse] Skipping line {} due to invalid duration '{}'", line_number, duration_str);
                continue;
            }
        };

        records.push(RttmRecord { start_time, duration, speaker_id });
    }
    info!("[RTTM Parse] Parsed {} SPEAKER records from {}", records.len(), rttm_path.display());
    Ok(records)
}

// --- Helper: Merge Diarization Results ---
fn merge_diarization_results(
    whisper_segments: &mut Vec<TranscriptSegment>, // These are plain text segments
    rttm_records: &[RttmRecord])
{
    if rttm_records.is_empty() {
        info!("[Merge] No RTTM records provided for merging.");
        return;
    }
    if whisper_segments.is_empty() {
        info!("[Merge] No whisper segments provided for merging.");
        return;
    }

    info!("[Merge] Merging {} whisper segments with {} RTTM speaker turns...", whisper_segments.len(), rttm_records.len());

    let mut sorted_rttm = rttm_records.to_vec();
    sorted_rttm.sort_by(|a, b| a.start_time.partial_cmp(&b.start_time).unwrap_or(CmpOrdering::Equal));

    let mut rttm_index = 0;

    for whisper_seg in whisper_segments.iter_mut() {
        let whisper_start = whisper_seg.start_time;
        let whisper_end = whisper_seg.end_time;

        if whisper_end <= whisper_start {
            warn!("[Merge] Skipping invalid whisper segment with start >= end: {:.3}s - {:.3}s", whisper_start, whisper_end);
            continue;
        }

        // Advance RTTM index past records that end before the current whisper segment starts
        while rttm_index < sorted_rttm.len() {
            let rttm_rec = &sorted_rttm[rttm_index];
            let rttm_turn_end = rttm_rec.start_time + rttm_rec.duration;
            if rttm_turn_end <= whisper_start {
                rttm_index += 1;
            } else {
                break; // Found a potentially overlapping RTTM record or one that starts later
            }
        }

        let mut speaker_overlaps: HashMap<String, f64> = HashMap::new();
        let mut speaker_contains_midpoint: Option<String> = None;
        let whisper_mid_point = whisper_start + (whisper_end - whisper_start) / 2.0;

        // Check RTTM records starting from the current rttm_index
        for i in rttm_index..sorted_rttm.len() {
            let rttm_rec = &sorted_rttm[i];
            let rttm_start = rttm_rec.start_time;
            let rttm_end = rttm_rec.start_time + rttm_rec.duration;

            // If RTTM record starts after whisper segment ends, no further RTTM records will overlap
            if rttm_start >= whisper_end {
                break; 
            }

            let overlap_start = whisper_start.max(rttm_start);
            let overlap_end = whisper_end.min(rttm_end);
            let overlap_duration = (overlap_end - overlap_start).max(0.0);

            if overlap_duration > 0.0 {
                *speaker_overlaps.entry(rttm_rec.speaker_id.clone()).or_insert(0.0) += overlap_duration;
            }

            // Check if this RTTM record contains the midpoint of the whisper segment
            if speaker_contains_midpoint.is_none() && whisper_mid_point >= rttm_start && whisper_mid_point < rttm_end {
                speaker_contains_midpoint = Some(rttm_rec.speaker_id.clone());
            }
        }
        
        // Assign speaker based on max overlap, then midpoint, then keep original
        if let Some((dominant_speaker, max_overlap)) = speaker_overlaps.into_iter().max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(CmpOrdering::Equal)) {
             debug!("[Merge] Assigning '{}' (overlap {:.3}s) to seg {:.3}-{:.3}", dominant_speaker, max_overlap, whisper_start, whisper_end);
            whisper_seg.speaker = dominant_speaker;
        } else if let Some(midpoint_speaker) = speaker_contains_midpoint {
             warn!("[Merge] No overlap found for seg {:.3}-{:.3}. Using midpoint speaker '{}'", whisper_start, whisper_end, midpoint_speaker);
            whisper_seg.speaker = midpoint_speaker;
        } else {
            // If no overlap and no midpoint speaker, keep the existing speaker (which defaults to "Unknown" or previous assignment)
            debug!("[Merge] No overlap or midpoint speaker found for seg {:.3}-{:.3}. Keeping original speaker '{}'.", whisper_start, whisper_end, whisper_seg.speaker);
        }
    }
    info!("[Merge] Finished merging diarization results.");
}

// --- Helper: Find Model File ---
fn find_model_file(model_dir: &Path) -> Result<PathBuf, CommandError> {
    debug!("[Helper] Searching for model file in directory: {:?}", model_dir);
    if !model_dir.exists() || !model_dir.is_dir() {
        return Err(CommandError::from(format!("Model directory not found or is not a directory: {}", model_dir.display())));
    }

    for entry_result in fs::read_dir(model_dir)? {
        match entry_result {
            Ok(entry) => {
                let path = entry.path();
                if path.is_file() {
                    if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                        let lower_ext = ext.to_lowercase();
                        if lower_ext == "bin" || lower_ext == "gguf" || lower_ext == "pt" { // Common model file extensions
                            info!("[Helper] Found potential model file: {:?}", path);
                            return Ok(path);
                        }
                    }
                }
            }
            Err(e) => {
                warn!("[Helper] Failed to read directory entry in '{}': {}", model_dir.display(), e);
            }
        }
    }

    Err(CommandError::from(format!("No model file (.bin, .gguf, .pt) found within directory: {}", model_dir.display())))
}

// --- cancel_transcription Command ---
#[tauri::command]
pub async fn cancel_transcription(
    job_id: String,
    cancel_state: State<'_, TranscriptionCancellationState>)
-> Result<(), CommandError> {
    info!("[Transcription] Received cancellation request for job: {}", job_id);
    if let Some(flag_entry) = cancel_state.0.get(&job_id) {
        let cancel_flag = flag_entry.value();
        match cancel_flag.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst) {
             Ok(_) => { info!("[Transcription] Cancellation flag successfully SET for job: {}", job_id); }
             Err(_) => { info!("[Transcription] Cancellation flag was already SET for job: {}", job_id); }
        }
    } else {
        warn!("[Transcription] Cancellation request for unknown or already completed job ID: {}", job_id);
    }
    Ok(())
}

// --- Helper: Emit Progress ---
pub(crate) async fn emit_progress(
    app_handle: &AppHandle,
    job_id: &str,
    percent: f32,
    message: &str)
-> Result<(), tauri::Error> {
    let clamped_percent = percent.max(0.0).min(100.0);
    debug!("[Progress Emit][{}] {:.1}% - {}", job_id, clamped_percent, message);
    app_handle.emit(
        "TRANSCRIPTION_PROGRESS",
        ProgressPayload {
            job_id: job_id.to_string(),
            percent: clamped_percent,
            message: message.to_string(),
        }
    )
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