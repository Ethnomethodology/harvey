// src-tauri/src/projectview/local_handler/transcription.rs

use crate::projectview::shared_types::{ProgressPayload, TranscriptSegment, TranscriptionResult};
use crate::projectview::shared_utils::get_project_xml_path_from_item;
use crate::projectview::transcription_commands::{
    create_lexical_paragraph_json_value, create_lexical_table_from_segments,
    map_speaker_ids_to_names, prepare_output_paths, save_transcript_json,
};
use crate::transcription::{
    faster_whisper::FasterWhisperEngine, whisper_cpp::WhisperCppEngine, TranscriptionEngine,
    TranscriptionOptions,
};
use crate::welcome::config::{get_default_download_location, read_config, CommandError};
use crate::welcome::python_env::{get_env_command, get_python_command};
use serde_json;

use log::{debug, error, info, warn};
use std::{
    fs::{self, File},
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};
use tauri::{AppHandle, Emitter, Manager, Runtime};
use tauri_plugin_shell::process::CommandEvent;
use tokio::time::sleep;
use uuid::Uuid;

use crate::projectview::utils::get_ffmpeg_path;

// Removed old Whisper JSON structs - using Engines instead

#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct TranscriptionJobCompletedPayload {
    job_id: String,
    status: String,
    job_finished_path: String,
    transcript_file_path: Option<String>,
    translated_transcript_file_path: Option<String>,
    error_message: Option<String>,
}

#[derive(Debug, Clone)]
struct RttmRecord {
    start_time: f64,
    duration: f64,
    speaker_id: String,
}

#[tauri::command]
pub async fn run_transcription<R: Runtime>(
    app_handle: AppHandle<R>,
    media_path: String,
    model_name: String,
    language: String,
    num_speakers: usize,
    diarization_on: bool,
    speaker_names: Vec<String>,
) -> Result<TranscriptionResult, CommandError> {
    let internal_job_id = Uuid::new_v4().to_string();
    info!(
        "[Transcription][LocalRun][{}] Start: Media='{}', Model='{}', Lang='{}', Speakers={}",
        internal_job_id, media_path, model_name, language, num_speakers
    );

    let cancel_flag = Arc::new(AtomicBool::new(false));

    if cancel_flag.load(Ordering::Relaxed) {
        warn!(
            "[Transcription][LocalRun][{}] Cancelled before starting any processing.",
            internal_job_id
        );
        let _ = app_handle.emit(
            "custom_transcription_job_completed",
            TranscriptionJobCompletedPayload {
                job_id: internal_job_id.clone(),
                status: "cancelled".to_string(),
                job_finished_path: media_path.clone(),
                transcript_file_path: None,
                translated_transcript_file_path: None,
                error_message: Some(
                    "Transcription cancelled by user before WAV conversion.".to_string(),
                ),
            },
        );
        return Err(CommandError::from(
            "Transcription cancelled by user before WAV conversion.",
        ));
    }

    let project_xml_path_buf = get_project_xml_path_from_item(&PathBuf::from(&media_path))?;
    let project_xml_path_str = project_xml_path_buf.to_string_lossy().to_string();
    info!(
        "[Transcription][LocalRun][{}] Found Project XML: {}",
        internal_job_id, project_xml_path_str
    );

    let _ = emit_progress(&app_handle, &internal_job_id, 1.0, "Preparing audio...").await;

    let wav_media_path_result =
        convert_to_wav_if_needed(&app_handle, &media_path, &internal_job_id, &cancel_flag).await;
    let wav_media_path = match wav_media_path_result {
        Ok(path) => path,
        Err(e) => {
            let error_message = format!("{}", e);
            if error_message.to_lowercase().contains("cancel") {
                warn!("[Transcription][LocalRun][{}] WAV conversion was cancelled. Emitting cancelled event.", internal_job_id);
                let _ = app_handle.emit(
                    "custom_transcription_job_completed",
                    TranscriptionJobCompletedPayload {
                        job_id: internal_job_id.clone(),
                        status: "cancelled".to_string(),
                        job_finished_path: media_path.clone(),
                        transcript_file_path: None,
                        translated_transcript_file_path: None,
                        error_message: Some(error_message.clone()),
                    },
                );
            }
            return Err(CommandError::from(format!(
                "WAV conversion failed: {}",
                error_message
            )));
        }
    };
    let _ = emit_progress(&app_handle, &internal_job_id, 4.0, "Audio ready.").await;
    info!(
        "[Transcription][LocalRun][{}] Using WAV path: {}",
        internal_job_id,
        wav_media_path.display()
    );

    let whisper_model_path_str = resolve_whisper_model_path(&model_name, &internal_job_id).await?;
    debug!(
        "[Transcription][LocalRun][{}] Whisper model path: '{}'",
        internal_job_id, whisper_model_path_str
    );

    let (
        output_path_base_str,
        expected_whisper_output_path,
        expected_rttm_path,
        final_transcript_path,
        _,
        _,
        _,
    ) = prepare_output_paths(&wav_media_path.to_string_lossy(), &internal_job_id, false)?;
    debug!(
        "[Transcription][LocalRun][{}] Paths - Base:'{}', Whisper:'{}', RTTM:'{}', Final:'{}'",
        internal_job_id,
        output_path_base_str,
        expected_whisper_output_path.display(),
        expected_rttm_path.display(),
        final_transcript_path.display()
    );

    if cancel_flag.load(Ordering::Relaxed) {
        warn!(
            "[Transcription][LocalRun][{}] Cancelled before Whisper processing.",
            internal_job_id
        );
        if wav_media_path.to_string_lossy() != media_path
            && wav_media_path.extension().is_some_and(|ext| ext == "wav")
        {
            let _ = fs::remove_file(&wav_media_path);
            info!(
                "[Transcription][LocalRun][{}] Cleaned up temporary WAV file: {}",
                internal_job_id,
                wav_media_path.display()
            );
        }
        let _ = app_handle.emit(
            "custom_transcription_job_completed",
            TranscriptionJobCompletedPayload {
                job_id: internal_job_id.clone(),
                status: "cancelled".to_string(),
                job_finished_path: media_path.clone(),
                transcript_file_path: None,
                translated_transcript_file_path: None,
                error_message: Some("Transcription cancelled by user before Whisper.".to_string()),
            },
        );
        return Err(CommandError::from(
            "Transcription cancelled by user before Whisper processing.",
        ));
    }

    let _ = emit_progress(
        &app_handle,
        &internal_job_id,
        5.0,
        "Running transcription...",
    )
    .await;

    // --- NEW: Use the Engine Architecture ---
    let config = read_config().unwrap_or_default();
    let engine_type = config
        .selected_transcription_engine
        .clone()
        .unwrap_or_else(|| "faster-whisper".to_string());

    let options = TranscriptionOptions {
        language_code: if language == "auto" {
            None
        } else {
            Some(language.clone())
        },
        model_path: whisper_model_path_str.clone(),
        output_dir: expected_whisper_output_path
            .parent()
            .unwrap_or(Path::new(""))
            .to_path_buf(),
        translate: false, // Default for now
        initial_prompt: None,
        hotwords: None,
    };

    let mut whisper_segments_plain = match engine_type.as_str() {
        "whisper-cpp" => {
            let engine = WhisperCppEngine::new(app_handle.clone());
            engine
                .transcribe(
                    &wav_media_path,
                    &options,
                    &internal_job_id,
                    cancel_flag.clone(),
                )
                .await?
        }
        _ => {
            let engine = FasterWhisperEngine::new(app_handle.clone());
            engine
                .transcribe(
                    &wav_media_path,
                    &options,
                    &internal_job_id,
                    cancel_flag.clone(),
                )
                .await?
        }
    };

    let _ = emit_progress(
        &app_handle,
        &internal_job_id,
        45.0,
        "Transcription finished.",
    )
    .await;

    if cancel_flag.load(Ordering::Relaxed) {
        warn!(
            "[Transcription][LocalRun][{}] Cancelled before Diarization processing.",
            internal_job_id
        );
        if expected_whisper_output_path.exists() {
            let _ = fs::remove_file(&expected_whisper_output_path);
            info!(
                "[Transcription][LocalRun][{}] Cleaned up whisper output: {}",
                internal_job_id,
                expected_whisper_output_path.display()
            );
        }
        if wav_media_path.to_string_lossy() != media_path
            && wav_media_path.extension().is_some_and(|ext| ext == "wav")
        {
            let _ = fs::remove_file(&wav_media_path);
            info!(
                "[Transcription][LocalRun][{}] Cleaned up temporary WAV file: {}",
                internal_job_id,
                wav_media_path.display()
            );
        }
        let _ = app_handle.emit(
            "custom_transcription_job_completed",
            TranscriptionJobCompletedPayload {
                job_id: internal_job_id.clone(),
                status: "cancelled".to_string(),
                job_finished_path: media_path.clone(),
                transcript_file_path: None,
                translated_transcript_file_path: None,
                error_message: Some(
                    "Transcription cancelled by user before Diarization.".to_string(),
                ),
            },
        );
        return Err(CommandError::from(
            "Transcription cancelled by user before Diarization processing.",
        ));
    }

    let rttm_records: Option<Vec<RttmRecord>> = if diarization_on {
        let _ = emit_progress(
            &app_handle,
            &internal_job_id,
            50.0,
            "Running diarization...",
        )
        .await;
        match run_python_diarization(
            &app_handle,
            &wav_media_path.to_string_lossy(),
            num_speakers,
            &expected_rttm_path,
            &internal_job_id,
            &cancel_flag,
        )
        .await
        {
            Ok(rttm_path) => match parse_rttm_file(&rttm_path) {
                Ok(records) => {
                    debug!(
                        "[Transcription][LocalRun][{}] Diarization success, {} turns.",
                        internal_job_id,
                        records.len()
                    );
                    let _ =
                        emit_progress(&app_handle, &internal_job_id, 85.0, "Merging results...")
                            .await;
                    Some(records)
                }
                Err(e) => {
                    error!(
                        "[Transcription][LocalRun][{}] Failed parse RTTM '{}': {}",
                        internal_job_id,
                        rttm_path.display(),
                        e
                    );
                    let _ =
                        emit_progress(&app_handle, &internal_job_id, 85.0, "RTTM parse failed.")
                            .await;
                    None
                }
            },
            Err(e) => {
                let error_message = format!("{}", e);
                if error_message.to_lowercase().contains("cancel") {
                    warn!("[Transcription][LocalRun][{}] Diarization was cancelled. Emitting cancelled event.", internal_job_id);
                    if expected_rttm_path.exists() {
                        let _ = fs::remove_file(&expected_rttm_path);
                        info!(
                            "[Transcription][LocalRun][{}] Cleaned up RTTM output: {}",
                            internal_job_id,
                            expected_rttm_path.display()
                        );
                    }
                    if expected_whisper_output_path.exists() {
                        let _ = fs::remove_file(&expected_whisper_output_path);
                        info!(
                            "[Transcription][LocalRun][{}] Cleaned up whisper output: {}",
                            internal_job_id,
                            expected_whisper_output_path.display()
                        );
                    }
                    if wav_media_path.to_string_lossy() != media_path
                        && wav_media_path.extension().is_some_and(|ext| ext == "wav")
                    {
                        let _ = fs::remove_file(&wav_media_path);
                        info!(
                            "[Transcription][LocalRun][{}] Cleaned up temporary WAV file: {}",
                            internal_job_id,
                            wav_media_path.display()
                        );
                    }
                    let _ = app_handle.emit(
                        "custom_transcription_job_completed",
                        TranscriptionJobCompletedPayload {
                            job_id: internal_job_id.clone(),
                            status: "cancelled".to_string(),
                            job_finished_path: media_path.clone(),
                            transcript_file_path: None,
                            translated_transcript_file_path: None,
                            error_message: Some(error_message.clone()),
                        },
                    );
                    return Err(CommandError::from(error_message));
                } else {
                    error!(
                        "[Transcription][LocalRun][{}] Diarization failed: {}.",
                        internal_job_id, error_message
                    );
                    warn!("Diarization script failed. Ensure the Python environment is correctly set up and `pyannote.audio` is installed.");
                    let _ =
                        emit_progress(&app_handle, &internal_job_id, 85.0, "Diarization failed.")
                            .await;
                    None
                }
            }
        }
    } else {
        info!(
            "[Transcription][LocalRun][{}] Skipping diarization (num_speakers=0).",
            internal_job_id
        );
        let _ = emit_progress(&app_handle, &internal_job_id, 85.0, "Skipping diarization.").await;
        None
    };

    if let Some(rttm_data) = &rttm_records {
        if !rttm_data.is_empty() {
            debug!(
                "[Transcription][LocalRun][{}] Merging diarization results...",
                internal_job_id
            );
            merge_diarization_results(&mut whisper_segments_plain, rttm_data);
            let _ = emit_progress(
                &app_handle,
                &internal_job_id,
                90.0,
                "Mapping speaker names...",
            )
            .await;
            map_speaker_ids_to_names(&mut whisper_segments_plain, &speaker_names);
        } else {
            warn!(
                "[Transcription][LocalRun][{}] Diarization ran but resulted in 0 RTTM records.",
                internal_job_id
            );
            let _ = emit_progress(
                &app_handle,
                &internal_job_id,
                90.0,
                "No speaker segments found.",
            )
            .await;
        }
    } else {
        debug!(
            "[Transcription][LocalRun][{}] No RTTM data to merge.",
            internal_job_id
        );
    }

    let _ = emit_progress(
        &app_handle,
        &internal_job_id,
        95.0,
        "Saving final transcript...",
    )
    .await;

    let lexical_table_json_value = create_lexical_table_from_segments(&whisper_segments_plain);
    let lexical_table_json_string = serde_json::to_string_pretty(&lexical_table_json_value)
        .map_err(|e| {
            CommandError::from(format!("Failed to serialize Lexical Table JSON: {}", e))
        })?;

    debug!(
        "[Transcription][LocalRun][{}] Saving final Lexical Table JSON to: {:?}",
        internal_job_id, final_transcript_path
    );
    save_transcript_json(
        project_xml_path_str,
        final_transcript_path.to_string_lossy().to_string(),
        lexical_table_json_string,
        Some(language.clone()), // Pass the original language code
    )
    .await?;
    info!(
        "[Transcription][LocalRun][{}] Final transcript saved.",
        internal_job_id
    );

    let segments_for_frontend_result: Vec<TranscriptSegment> = whisper_segments_plain
        .iter()
        .cloned()
        .map(|seg_plain| {
            let cell_content_lexical_value = create_lexical_paragraph_json_value(&seg_plain.text);
            let cell_content_lexical_string = serde_json::to_string(&cell_content_lexical_value)
                .unwrap_or_else(|_| {
                    serde_json::to_string(&create_lexical_paragraph_json_value("")).unwrap()
                });

            TranscriptSegment {
                start_time: seg_plain.start_time,
                end_time: seg_plain.end_time,
                speaker: seg_plain.speaker.clone(),
                text: cell_content_lexical_string,
                words: seg_plain.words.clone(),
            }
        })
        .collect();

    info!(
        "[Transcription][LocalRun][{}] Process complete.",
        internal_job_id
    );
    let _ = emit_progress(
        &app_handle,
        &internal_job_id,
        100.0,
        "Transcription complete.",
    )
    .await;

    let _ = app_handle.emit(
        "custom_transcription_job_completed",
        TranscriptionJobCompletedPayload {
            job_id: internal_job_id.clone(),
            status: "done".to_string(),
            job_finished_path: media_path.clone(),
            transcript_file_path: Some(final_transcript_path.to_string_lossy().into_owned()),
            translated_transcript_file_path: None,
            error_message: None,
        },
    );

    Ok(TranscriptionResult {
        segments: segments_for_frontend_result,
        transcript_file_path: final_transcript_path.to_string_lossy().into_owned(),
    })
}

// --- Helper: Convert to WAV using FFmpeg ---
pub(crate) async fn convert_to_wav_if_needed<R: Runtime>(
    app_handle: &AppHandle<R>,
    input_path_str: &str,
    job_id: &str, // Now internal_job_id from caller
    cancel_flag: &Arc<AtomicBool>,
) -> Result<PathBuf, CommandError> {
    info!(
        "[FFmpeg][{}] Checking audio file: {}",
        job_id, input_path_str
    );
    let input_path = PathBuf::from(input_path_str);
    let extension = input_path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();

    if extension == "wav" {
        info!(
            "[FFmpeg][{}] Input is already WAV. Skipping conversion.",
            job_id
        );
        return Ok(input_path);
    }

    let output_wav_path = input_path.with_extension("wav");
    info!(
        "[FFmpeg][{}] Target WAV path: {}",
        job_id,
        output_wav_path.display()
    );

    if output_wav_path.exists() {
        match output_wav_path.metadata() {
            Ok(m) if m.len() > 0 => {
                info!(
                    "[FFmpeg][{}] Target WAV file already exists and is not empty. Reusing.",
                    job_id
                );
                return Ok(output_wav_path);
            }
            _ => {
                warn!("[FFmpeg][{}] Target WAV file exists but is empty or metadata error. Overwriting.", job_id);
            }
        }
    }

    info!("[FFmpeg][{}] Starting FFmpeg conversion...", job_id);
    let _ = emit_progress(app_handle, job_id, 2.0, "Converting audio to WAV...").await;

    let ffmpeg_path = get_ffmpeg_path(app_handle)?;

    let args: Vec<String> = vec![
        "-i".into(),
        input_path_str.to_string(),
        "-vn".into(),
        "-acodec".into(),
        "pcm_s16le".into(),
        "-ar".into(),
        "16000".into(),
        "-ac".into(),
        "1".into(),
        "-y".into(),
        output_wav_path.to_string_lossy().to_string(),
    ];
    debug!("[FFmpeg][{}] Command arguments: {:?}", job_id, args);

    let (mut rx, child) = get_env_command(app_handle, &ffmpeg_path.to_string_lossy())?
        .args(args)
        .spawn()?;
    debug!(
        "[FFmpeg][{}] Spawned FFmpeg process (PID: {:?})",
        job_id,
        child.pid()
    );

    let mut ffmpeg_stderr: Vec<String> = Vec::new();
    let mut ffmpeg_exit_code: Option<i32> = None;
    let mut ffmpeg_error: Option<String> = None;

    loop {
        if cancel_flag.load(Ordering::Relaxed) {
            warn!(
                "[FFmpeg][{}] Cancellation requested. Killing FFmpeg process...",
                job_id
            );
            let _ = child.kill();
            if output_wav_path.exists() {
                let _ = fs::remove_file(&output_wav_path);
            }
            return Err(CommandError::from("Audio conversion cancelled."));
        }

        tokio::select! {
            biased;
            maybe_event = rx.recv() => {
                match maybe_event {
                    Some(event) => match event {
                        CommandEvent::Stdout(line) => { debug!("[FFmpeg][stdout][{}] {}", job_id, String::from_utf8_lossy(&line).trim_end()); },
                        CommandEvent::Stderr(line) => { let l = String::from_utf8_lossy(&line).to_string(); debug!("[FFmpeg][stderr][{}] {}", job_id, l.trim_end()); ffmpeg_stderr.push(l); },
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
        error!(
            "[FFmpeg][{}] FFmpeg process failed. Code: {:?}, Error: {:?}\nStderr:\n{}",
            job_id, ffmpeg_exit_code, ffmpeg_error, stderr_output
        );
        if output_wav_path.exists() {
            let _ = fs::remove_file(&output_wav_path);
        }
        return Err(CommandError::from(format!(
            "FFmpeg conversion failed. Code: {:?}. Error: {}",
            ffmpeg_exit_code,
            ffmpeg_error.unwrap_or_default()
        )));
    }

    if !output_wav_path.exists() {
        error!(
            "[FFmpeg][{}] FFmpeg reported success, but output file is missing: {}",
            job_id,
            output_wav_path.display()
        );
        return Err(CommandError::from(format!(
            "FFmpeg conversion failed: output file missing ({})",
            output_wav_path.display()
        )));
    }
    match output_wav_path.metadata() {
        Ok(m) if m.len() == 0 => {
            error!(
                "[FFmpeg][{}] FFmpeg reported success, but output file is empty: {}",
                job_id,
                output_wav_path.display()
            );
            let _ = fs::remove_file(&output_wav_path);
            return Err(CommandError::from(format!(
                "FFmpeg conversion failed: output file is empty ({})",
                output_wav_path.display()
            )));
        }
        Err(e) => {
            error!(
                "[FFmpeg][{}] FFmpeg reported success, but failed to get metadata for {}: {}",
                job_id,
                output_wav_path.display(),
                e
            );
            let _ = fs::remove_file(&output_wav_path);
            return Err(CommandError::from(format!(
                "FFmpeg conversion failed: output metadata error ({})",
                e
            )));
        }
        Ok(_) => {}
    }

    info!(
        "[FFmpeg][{}] Successfully converted '{}' to WAV: {}",
        job_id,
        input_path_str,
        output_wav_path.display()
    );
    Ok(output_wav_path)
}

// --- Helper: Resolve Whisper Model Path ---
async fn resolve_whisper_model_path(
    model_name: &str,
    job_id: &str,
) -> Result<String, CommandError> {
    // job_id is internal_job_id
    let config = read_config()?;
    let base_model_dir_str = if !config.download_location.trim().is_empty() {
        config.download_location
    } else {
        get_default_download_location()?
    };
    let model_dir_path = PathBuf::from(&base_model_dir_str).join(model_name);

    if !model_dir_path.exists() || !model_dir_path.is_dir() {
        let e = format!(
            "Model directory not found: '{}'. Please download the model first.",
            model_dir_path.display()
        );
        error!(
            "[Transcription][LocalRun][{}] Error resolving model path: {}",
            job_id, e
        );
        return Err(CommandError::from(e));
    }
    let model_file_path = find_model_file(&model_dir_path)?;
    Ok(model_file_path.to_string_lossy().to_string())
}

// Removed redundant sidecar and parsing functions

async fn run_python_diarization<R: Runtime>(
    app_handle: &AppHandle<R>,
    media_path: &str,
    num_speakers: usize,
    output_rttm_path: &Path,
    job_id: &str,
    cancel_flag: &Arc<AtomicBool>,
) -> Result<PathBuf, CommandError> {
    info!(
        "[PyDiarize][{}] Starting diarization for: {}",
        job_id, media_path
    );
    if let Some(parent_dir) = output_rttm_path.parent() {
        fs::create_dir_all(parent_dir)?;
    } else {
        return Err(CommandError::from(format!(
            "Could not get parent directory for RTTM output: {}",
            output_rttm_path.display()
        )));
    }

    let script_path = app_handle
        .path()
        .resolve(
            "scripts/run_diarization.py",
            tauri::path::BaseDirectory::Resource,
        )
        .map_err(|e| CommandError::from(e.to_string()))?;

    // Read the Hugging Face token
    let config_dir = app_handle
        .path()
        .app_config_dir()
        .map_err(|_| CommandError::from("Failed to get app config dir"))?;
    let token_path = config_dir.join("hf_token");
    let token = fs::read_to_string(token_path).map_err(|e| {
        CommandError::from(format!(
            "Failed to read Hugging Face token: {}. Please save it in the configuration.",
            e
        ))
    })?;

    let args = vec![
        script_path.to_string_lossy().to_string(),
        media_path.to_string(),
        num_speakers.to_string(),
        token,
    ];

    debug!(
        "[PyDiarize][{}] Running script '{}'",
        job_id,
        script_path.display()
    );
    let mut command = get_python_command(app_handle)?;

    if let Ok(hf_home) = crate::welcome::diarization::get_diarization_hub_path(app_handle) {
        command = command.env("HF_HOME", hf_home.to_string_lossy().to_string());
    }

    let (mut rx, child) = command
        .args(args)
        .spawn()
      .map_err(|e| {
          error!("Failed to spawn Python script: {}. Ensure Python environment and pyannote.audio are set up.", e);
          CommandError::from(format!("Failed to execute Python diarization script: {}.", e))
      })?;
    debug!(
        "[PyDiarize][{}] Spawned Python process (PID: {:?})",
        job_id,
        child.pid()
    );

    let mut stderr_lines: Vec<String> = Vec::new();
    let mut stdout_lines: Vec<String> = Vec::new();
    let mut process_error: Option<String> = None;
    let mut exit_code: Option<i32> = None;

    loop {
        if cancel_flag.load(Ordering::Relaxed) {
            warn!(
                "[PyDiarize][{}] Cancellation requested. Killing process...",
                job_id
            );
            let _ = child.kill();
            if output_rttm_path.exists() {
                let _ = fs::remove_file(output_rttm_path);
            }
            return Err(CommandError::from("Diarization process cancelled."));
        }

        tokio::select! {
            biased;
            maybe_event = rx.recv() => {
                match maybe_event {
                    Some(event) => match event {
                        CommandEvent::Stdout(line) => { let l = String::from_utf8_lossy(&line); stdout_lines.push(l.to_string()); },
                        CommandEvent::Stderr(line) => { let l = String::from_utf8_lossy(&line); debug!("[PyDiarize][stderr][{}] {}", job_id, l.trim_end()); stderr_lines.push(l.to_string()); }
                        CommandEvent::Error(msg) => { error!("[PyDiarize][error][{}] {}", job_id, msg); process_error = Some(msg); break; }
                        CommandEvent::Terminated(payload) => { info!("[PyDiarize][term][{}] Process terminated. Code: {:?}, Signal: {:?}", job_id, payload.code, payload.signal); exit_code = payload.code; if payload.signal.is_some() && exit_code.is_none() { exit_code = Some(-1); } break; }
                        _ => {}
                    },
                    None => { if exit_code.is_none() && process_error.is_none() { warn!("[PyDiarize][{}] Event channel closed unexpectedly.", job_id); exit_code = Some(-1); } break; }
                }
            }
            _ = sleep(Duration::from_millis(100)) => { continue; }
        }
    }

    let rttm_output = stdout_lines.join("");
    let final_stderr = stderr_lines.join("\n");
    info!(
        "[PyDiarize][{}] Python script finished. Code: {:?}, Error: {:?}.",
        job_id, exit_code, process_error
    );

    if !final_stderr.is_empty() {
        if process_error.is_some() || exit_code != Some(0) {
            error!("[PyDiarize][{}] Stderr:\n{}", job_id, final_stderr);
        } else {
            debug!("[PyDiarize][{}] Stderr:\n{}", job_id, final_stderr);
        }
    }

    if process_error.is_some() || exit_code != Some(0) {
        let ec_str = exit_code.map_or("N/A".to_string(), |c| c.to_string());
        let error_message = format!(
            "Python script failed. Exit Code: {}. Error: {}. Stderr: {}",
            ec_str,
            process_error.unwrap_or_default(),
            final_stderr.chars().take(500).collect::<String>()
        );
        error!("[PyDiarize][{}] Error: {}", job_id, error_message);
        if output_rttm_path.exists() {
            let _ = fs::remove_file(output_rttm_path);
        }
        return Err(CommandError::from(error_message));
    }

    // Write the captured stdout (RTTM content) to the output file
    fs::write(output_rttm_path, rttm_output)?;

    match output_rttm_path.metadata() {
        Ok(m) if m.len() == 0 => {
            warn!(
                "[PyDiarize][{}] Output RTTM file exists but is empty: {:?}",
                job_id, output_rttm_path
            );
        }
        Err(e) => {
            error!(
                "[PyDiarize][{}] Failed to get metadata for RTTM output file {}: {}",
                job_id,
                output_rttm_path.display(),
                e
            );
            let _ = fs::remove_file(output_rttm_path);
            return Err(CommandError::from(format!(
                "RTTM output file validation error: {}",
                e
            )));
        }
        Ok(_) => {}
    }

    info!(
        "[PyDiarize][{}] RTTM file created successfully: {:?}",
        job_id, output_rttm_path
    );
    Ok(output_rttm_path.to_path_buf())
}

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
            warn!(
                "[RTTM Parse] Skipping malformed line {} ({} fields): '{}'",
                line_number,
                parts.len(),
                line
            );
            continue;
        }

        if parts.first().is_some_and(|&t| t != "SPEAKER") {
            debug!(
                "[RTTM Parse] Skipping non-SPEAKER line {}: '{}'",
                line_number, line
            );
            continue;
        }

        let start_str = parts[3];
        let duration_str = parts[4];
        let speaker_id = parts[7].to_string();

        let start_time: f64 = match start_str.parse() {
            Ok(t) if t >= 0.0 => t,
            _ => {
                warn!(
                    "[RTTM Parse] Skipping line {} due to invalid start time '{}'",
                    line_number, start_str
                );
                continue;
            }
        };

        let duration: f64 = match duration_str.parse() {
            Ok(d) if d > 0.0 => d,
            _ => {
                warn!(
                    "[RTTM Parse] Skipping line {} due to invalid duration '{}'",
                    line_number, duration_str
                );
                continue;
            }
        };

        records.push(RttmRecord {
            start_time,
            duration,
            speaker_id,
        });
    }
    info!(
        "[RTTM Parse] Parsed {} SPEAKER records from {}",
        records.len(),
        rttm_path.display()
    );
    Ok(records)
}

fn merge_diarization_results(
    whisper_segments: &mut Vec<TranscriptSegment>,
    rttm_records: &[RttmRecord],
) {
    if whisper_segments.is_empty() {
        return;
    }

    // 1. Flatten all words into a single list
    let mut all_words = Vec::new();
    for seg in whisper_segments.iter() {
        if let Some(words) = &seg.words {
            if words.is_empty() {
                // Fallback: Dummy word for segment if word-level data is empty
                all_words.push(crate::projectview::shared_types::Word {
                    start: seg.start_time,
                    end: seg.end_time,
                    text: seg.text.clone(),
                    speaker: None,
                    probability: 1.0,
                });
            } else {
                for w in words {
                    all_words.push(w.clone());
                }
            }
        } else {
            // Fallback: If no word data (unlikely now), create a dummy word for the segment
            all_words.push(crate::projectview::shared_types::Word {
                start: seg.start_time,
                end: seg.end_time,
                text: seg.text.clone(),
                speaker: None,
                probability: 1.0,
            });
        }
    }

    if all_words.is_empty() {
        return;
    }

    // 2. Assign speakers to each word based on RTTM overlap
    if !rttm_records.is_empty() {
        info!(
            "[Merge] Mapping speakers to {} words using {} RTTM records...",
            all_words.len(),
            rttm_records.len()
        );
        for word in all_words.iter_mut() {
            let mut best_speaker = None;
            let mut max_overlap = 0.0;

            for rttm in rttm_records {
                let rttm_end = rttm.start_time + rttm.duration;
                let overlap_start = word.start.max(rttm.start_time);
                let overlap_end = word.end.min(rttm_end);
                let overlap = (overlap_end - overlap_start).max(0.0);

                if overlap > max_overlap {
                    max_overlap = overlap;
                    best_speaker = Some(rttm.speaker_id.clone());
                }
            }
            word.speaker = best_speaker;
        }
    }

    // 3. Re-cluster words into segments
    let mut new_segments = Vec::new();
    if all_words.is_empty() {
        return;
    }

    let mut current_segment_words = Vec::new();
    let mut current_speaker = all_words[0]
        .speaker
        .clone()
        .unwrap_or_else(|| "Unknown".to_string());

    for word in all_words {
        let word_speaker = word
            .speaker
            .clone()
            .unwrap_or_else(|| "Unknown".to_string());
        let last_word_end = current_segment_words
            .last()
            .map(|w: &crate::projectview::shared_types::Word| w.end)
            .unwrap_or(word.start);

        // Conditions for a new segment:
        // - Speaker changed
        // - Large silence gap (> 1.5s)
        let speaker_changed = word_speaker != current_speaker;
        let silence_gap = word.start - last_word_end > 1.5;

        if !current_segment_words.is_empty() && (speaker_changed || silence_gap) {
            // Finalize current segment
            new_segments.push(create_segment_from_words(
                current_segment_words,
                current_speaker,
            ));
            current_segment_words = Vec::new();
            current_speaker = word_speaker;
        }
        current_segment_words.push(word);
    }

    // Add the final segment
    if !current_segment_words.is_empty() {
        new_segments.push(create_segment_from_words(
            current_segment_words,
            current_speaker,
        ));
    }

    *whisper_segments = new_segments;
    info!(
        "[Merge] Re-clustered into {} segments.",
        whisper_segments.len()
    );
}

fn create_segment_from_words(
    words: Vec<crate::projectview::shared_types::Word>,
    speaker: String,
) -> TranscriptSegment {
    let start_time = words.first().map(|w| w.start).unwrap_or(0.0);
    let end_time = words.last().map(|w| w.end).unwrap_or(0.0);
    let text = words
        .iter()
        .map(|w| w.text.clone())
        .collect::<Vec<String>>()
        .join(" ");

    TranscriptSegment {
        start_time,
        end_time,
        speaker,
        text,
        words: Some(words),
    }
}

fn find_model_file(model_dir: &Path) -> Result<PathBuf, CommandError> {
    debug!(
        "[Helper] Searching for model file in directory: {:?}",
        model_dir
    );
    if !model_dir.exists() || !model_dir.is_dir() {
        return Err(CommandError::from(format!(
            "Model directory not found or is not a directory: {}",
            model_dir.display()
        )));
    }

    for entry_result in fs::read_dir(model_dir)? {
        match entry_result {
            Ok(entry) => {
                let path = entry.path();
                if path.is_file() {
                    if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                        let lower_ext = ext.to_lowercase();
                        if lower_ext == "bin" || lower_ext == "gguf" || lower_ext == "pt" {
                            info!("[Helper] Found potential model file: {:?}", path);
                            return Ok(path);
                        }
                    }
                }
            }
            Err(e) => {
                warn!(
                    "[Helper] Failed to read directory entry in '{}': {}",
                    model_dir.display(),
                    e
                );
            }
        }
    }

    Err(CommandError::from(format!(
        "No model file (.bin, .gguf, .pt) found within directory: {}",
        model_dir.display()
    )))
}

pub(crate) async fn emit_progress<R: Runtime>(
    app_handle: &AppHandle<R>,
    job_id: &str,
    percent: f32,
    message: &str,
) -> Result<(), tauri::Error> {
    let clamped_percent = percent.max(0.0).min(100.0);
    debug!(
        "[Progress Emit][{}] {:.1}% - {}",
        job_id, clamped_percent, message
    );
    app_handle.emit(
        "TRANSCRIPTION_PROGRESS",
        ProgressPayload {
            job_id: job_id.to_string(),
            percent: clamped_percent,
            message: message.to_string(),
        },
    )
}
