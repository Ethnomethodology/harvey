// src-tauri/src/projectview/transcription_handler.rs
use super::shared_types::{
    TranscriptSegment, ProjectXml, ImportedTranscriptEntryXml, FileMetadata,
    HARVEY_FILES_DIR, DOCS_DIR, TEMP_SUBDIR_DOCS, TRANSCRIPTS_DIR,
};
use super::shared_utils::{truncate_filename_stem, MAX_FILENAME_STEM_LENGTH, save_project_xml};
use crate::projectview::db_handler;
use crate::welcome::config::CommandError;
use regex::Regex;
use std::{
    fs,
    path::{Path, PathBuf}, // Path added back
    // time::{SystemTime, UNIX_EPOCH}, // Removed as timestamp is no longer in filename
};
use chrono::Utc; // Added for timestamping metadata
use tauri::{AppHandle, Runtime};
use tauri::Manager;
use tauri_plugin_shell::ShellExt;
use uuid::Uuid; // For temp file uniqueness
use crate::welcome::python_env::get_python_path;
use log::{debug, error, info, warn};

// Helper to convert HH:MM:SS to seconds
fn time_str_to_seconds(time_str: &str) -> Result<f64, CommandError> {
    let parts: Vec<&str> = time_str.split(':').collect();
    if parts.len() != 3 {
        return Err(CommandError::from(format!("Invalid time format: {}", time_str)));
    }
    let hours = parts[0].parse::<f64>().map_err(|_| CommandError::from(format!("Invalid hours in time string: {}", time_str)))?;
    let minutes = parts[1].parse::<f64>().map_err(|_| CommandError::from(format!("Invalid minutes in time string: {}", time_str)))?;
    let seconds = parts[2].parse::<f64>().map_err(|_| CommandError::from(format!("Invalid seconds in time string: {}", time_str)))?;
    Ok(hours * 3600.0 + minutes * 60.0 + seconds)
}

// Basic HTML tag stripper
fn strip_html_tags(html: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;
    for char_code in html.chars() {
        match char_code {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => result.push(char_code),
            _ => (),
        }
    }
    result
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .trim()
        .to_string()
}

fn parse_transcript_block(transcript_text_content: &str) -> Result<Vec<TranscriptSegment>, CommandError> {
    let mut segments: Vec<TranscriptSegment> = Vec::new();
    let mut current_segment: Option<TranscriptSegment> = None;

    let re_timestamp_speaker = Regex::new(r"^(\d{2}:\d{2}:\d{2})\s+(Speaker\s*\d+):?\s*$")
        .map_err(|e| CommandError::from(format!("Regex compilation for timestamp/speaker failed: {}", e)))?;
    let re_timestamp_only = Regex::new(r"^(\d{2}:\d{2}:\d{2})\s*$")
        .map_err(|e| CommandError::from(format!("Regex compilation for timestamp_only failed: {}", e)))?;

    for line in transcript_text_content.lines() {
        let trimmed_line = line.trim();
        debug!("[parse_transcript_block] Processing line: '{}'", trimmed_line);
        // Allow processing of empty lines if a segment is active, as they might be intentional newlines within text
        if trimmed_line.is_empty() {
            if let Some(ref mut seg) = current_segment {
                 if !seg.text.is_empty() && !seg.text.ends_with('\n') { // Avoid multiple newlines if already present
                    seg.text.push('\n');
                 }
            }
            continue;
        }

        if let Some(caps) = re_timestamp_speaker.captures(trimmed_line) {
            debug!("[parse_transcript_block] Matched timestamp+speaker line: '{}'", trimmed_line);
            let time_str = caps.get(1).unwrap().as_str();
            let speaker_str_raw = caps.get(2).unwrap().as_str();
            let speaker_str = speaker_str_raw.replace(char::is_whitespace, "-"); 
            let start_time = time_str_to_seconds(time_str)?;

            if let Some(mut seg) = current_segment.take() {
                seg.end_time = start_time; 
                seg.text = seg.text.trim().to_string();
                if !seg.text.is_empty() {
                    segments.push(seg);
                } else {
                    warn!("[Transcript Parse] Segment for speaker {} at {:.2}s had no text, discarding.", seg.speaker, seg.start_time);
                }
            }
            current_segment = Some(TranscriptSegment {
                start_time,
                end_time: 0.0, 
                speaker: speaker_str,
                text: String::new(),
            });
        } else if let Some(caps_time_only) = re_timestamp_only.captures(trimmed_line) {
            debug!("[parse_transcript_block] Matched timestamp only line: '{}'", trimmed_line);
            if let Some(mut seg) = current_segment.take() {
                let end_time_val = time_str_to_seconds(caps_time_only.get(1).unwrap().as_str())?;
                seg.end_time = end_time_val;
                seg.text = seg.text.trim().to_string();
                if !seg.text.is_empty() {
                     segments.push(seg);
                } else {
                    warn!("[Transcript Parse] Segment (ending at {}) had no text, discarding.", end_time_val);
                }
            }
        } else if let Some(ref mut seg) = current_segment {
            debug!("[parse_transcript_block] Appending to segment '{}': '{}'", seg.speaker, trimmed_line);
            if !seg.text.is_empty() && !seg.text.ends_with('\n') { 
                seg.text.push(' '); // Add space between consecutive text lines for the same speaker
            }
            seg.text.push_str(trimmed_line);
        } else {
            debug!("[Transcript Parse] Ignoring line (no current segment or not recognized format): {}", trimmed_line);
        }
    }

    if let Some(mut seg) = current_segment.take() {
        seg.text = seg.text.trim().to_string();
        if seg.end_time == 0.0 { 
            let calculated_end_time = seg.start_time + 300.0; 
            if let Some(last_segment) = segments.last() {
                 if seg.start_time >= last_segment.end_time {
                    seg.end_time = seg.start_time + 5.0; 
                 } else { 
                    seg.end_time = last_segment.end_time + 5.0; 
                    warn!("[Transcript Parse] Final segment start_time ({:.2}) <= previous end_time ({:.2}). Defaulting end_time.", seg.start_time, last_segment.end_time);
                 }
            } else {
                seg.end_time = calculated_end_time; // Use a larger default if it's the only segment
            }
             info!("[Transcript Parse] Setting end time for final segment ({}) to: {:.2}s", seg.speaker, seg.end_time);
        }
        if !seg.text.is_empty() {
            segments.push(seg);
        } else {
             warn!("[Transcript Parse] Final segment (speaker {}) had no text, discarding.", seg.speaker);
        }
    }
    if segments.is_empty() {
        warn!("[Transcript Parse] No valid transcript segments found after parsing.");
    }
    debug!("[parse_transcript_block] Finished parsing. Total segments: {}", segments.len());

    Ok(segments)
}


#[tauri::command]
pub async fn import_word_transcript<R: Runtime>(
    app_handle: AppHandle<R>,
    source_docx_path_str: String,
    project_xml_path_str: String,
) -> Result<String, CommandError> {
    info!("[import_word_transcript] Source DOCX: {}, Project XML: {}", source_docx_path_str, project_xml_path_str);
    let source_docx_path = PathBuf::from(&source_docx_path_str);
    let project_xml_path = PathBuf::from(&project_xml_path_str);

    if !source_docx_path.exists() {
        return Err(CommandError::from(format!("Source DOCX not found: {}", source_docx_path_str)));
    }
    let project_base_dir = project_xml_path.parent().ok_or_else(|| CommandError::from("Could not get project base dir from XML"))?;

    let original_docx_filename = source_docx_path.file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| CommandError::from("Could not get original DOCX filename"))?
        .to_string();

    // Truncate the original DOCX filename's stem to be used for the transcript's folder and JSON filename base.
    let truncated_transcript_base_name = truncate_filename_stem(&original_docx_filename, MAX_FILENAME_STEM_LENGTH);
    let transcript_filename_stem = Path::new(&truncated_transcript_base_name).file_stem() // Get stem from "truncated.docx"
        .and_then(|s| s.to_str())
        .unwrap_or("imported_transcript") // Fallback
        .to_string();
    info!("[import_word_transcript] Original DOCX: '{}', Truncated base for transcript: '{}'", original_docx_filename, transcript_filename_stem);

    let temp_html_dir = project_base_dir.join(HARVEY_FILES_DIR).join(DOCS_DIR).join(TEMP_SUBDIR_DOCS);
    fs::create_dir_all(&temp_html_dir).map_err(|e| CommandError::from(format!("Failed to create temp dir for HTML: {}", e)))?;
    let unique_id = Uuid::new_v4().to_string(); 
    // Use truncated stem for temp file to keep it shorter as well, though not strictly necessary for temp file.
    let temp_html_filename = format!("temp_transcript_html_{}_{}.html", transcript_filename_stem, unique_id);
    let temp_html_path = temp_html_dir.join(&temp_html_filename);

    let python_path = get_python_path()?;
    let script_path = app_handle.path()
        .resolve("scripts/convert_with_pandoc.py", tauri::path::BaseDirectory::Resource)
        .map_err(|e| CommandError::from(format!("Failed to resolve pandoc script path: {}", e)))?;

    let pandoc_args = vec![
        source_docx_path.to_string_lossy().to_string(),
        temp_html_path.to_string_lossy().to_string(),
        "html".to_string(),
    ];

    info!("[import_word_transcript] Pandoc CMD: {} {} {}", python_path.display(), script_path.display(), pandoc_args.join(" "));
    let (mut rx, _child) = app_handle.shell().command(python_path.to_string_lossy().to_string())
        .args(&[script_path.to_string_lossy().to_string()])
        .args(&pandoc_args)
        .spawn()
        .map_err(|e| CommandError::from(format!("Failed to spawn Pandoc script: {}", e)))?;

    let mut pandoc_output_stderr = String::new();
    let mut pandoc_success = false;
    while let Some(event) = rx.recv().await {
        match event {
            tauri_plugin_shell::process::CommandEvent::Stdout(line) => {
                debug!("[Pandoc Stdout] {}", String::from_utf8_lossy(&line).trim_end());
            }
            tauri_plugin_shell::process::CommandEvent::Stderr(line) => {
                let err_line = String::from_utf8_lossy(&line);
                warn!("[Pandoc Stderr] {}", err_line.trim_end());
                pandoc_output_stderr.push_str(&err_line);
            }
            tauri_plugin_shell::process::CommandEvent::Terminated(payload) => {
                if payload.code == Some(0) {
                    pandoc_success = true;
                } else {
                    error!("[import_word_transcript] Pandoc failed. Code: {:?}, Stderr: {}", payload.code, pandoc_output_stderr);
                }
                break;
            }
            tauri_plugin_shell::process::CommandEvent::Error(e) => {
                error!("[import_word_transcript] Pandoc execution error: {}", e);
                pandoc_output_stderr.push_str(&format!("\nExecution Error: {}", e));
                break;
            }
            _ => {}
        }
    }

    if !pandoc_success || !temp_html_path.exists() {
        let _ = fs::remove_file(&temp_html_path);
        return Err(CommandError::from(format!("Pandoc conversion failed. Stderr: {}", pandoc_output_stderr)));
    }

    let html_content = fs::read_to_string(&temp_html_path)
        .map_err(|e| CommandError::from(format!("Failed to read temp HTML: {}", e)))?;
    // Normalize <br> tags to newlines
    let html_content = html_content
        .replace("<br>", "\n")
        .replace("<br/>", "\n")
        .replace("<BR>", "\n")
        .replace("<BR/>", "\n");
    info!("[import_word_transcript] HTML content (first 200 chars): {}", html_content.chars().take(200).collect::<String>());
    let _ = fs::remove_file(&temp_html_path);

    let mut transcript_block_text_option: Option<String> = None;
    let mut in_transcript_section = false;
    let mut collected_lines_for_block = Vec::new();
    let transcript_heading_re = Regex::new(r"(?i)^\s*Transcript\s*$").unwrap();

    for line_raw in html_content.lines() {
        let line_stripped_of_tags = strip_html_tags(line_raw);
        
        if transcript_heading_re.is_match(&line_stripped_of_tags) {
            debug!("[HTML Parse] Found 'Transcript' heading: '{}'", line_stripped_of_tags);
            in_transcript_section = true;
            collected_lines_for_block.clear(); 
            continue; 
        }

        if in_transcript_section {
            // Collect all non-empty lines after stripping tags
            if !line_stripped_of_tags.is_empty() {
                collected_lines_for_block.push(line_stripped_of_tags);
            } else if !collected_lines_for_block.is_empty() { 
                // If we've already started collecting, an empty line after stripping might be a paragraph break
                collected_lines_for_block.push(String::new()); // Add an empty string to represent a newline
            }
        }
    }
    info!("[import_word_transcript] Collected {} lines under 'Transcript': {:?}", collected_lines_for_block.len(), collected_lines_for_block);
    
    if !collected_lines_for_block.is_empty() {
        transcript_block_text_option = Some(collected_lines_for_block.join("\n"));
    } else if in_transcript_section { 
        warn!("[import_word_transcript] 'Transcript' heading found, but no subsequent content collected.");
        transcript_block_text_option = Some(String::new()); 
    }
    
    let transcript_text_content = transcript_block_text_option.ok_or_else(|| CommandError::from("Could not find 'Transcript' section in the document. Please ensure a heading named 'Transcript' exists."))?;
    info!("[import_word_transcript] Transcript text content:\n{}", transcript_text_content);
    
    let segments = parse_transcript_block(&transcript_text_content)?;
    info!("[import_word_transcript] Parsed {} segments: {:#?}", segments.len(), segments);
    if segments.is_empty() {
        return Err(CommandError::from("No transcript segments were parsed. Check document format under 'Transcript' heading: expected 'HH:MM:SS Speaker X' lines followed by text."));
    }
    info!("[import_word_transcript] Parsed {} transcript segments.", segments.len());

    let target_standalone_transcripts_dir = project_base_dir.join(HARVEY_FILES_DIR).join(TRANSCRIPTS_DIR);
    fs::create_dir_all(&target_standalone_transcripts_dir)
        .map_err(|e| CommandError::from(format!("Failed to create standalone transcripts dir: {}", e)))?;

    // Create a dedicated subdirectory for this imported transcript, using the truncated stem
    let import_dir = target_standalone_transcripts_dir.join(&transcript_filename_stem); // Folder uses truncated stem
    fs::create_dir_all(&import_dir)
        .map_err(|e| CommandError::from(format!(
            "Failed to create imported transcript dir {}: {}", 
            import_dir.display(), e
        )))?;

    // Determine unique filename for the .json file, using the truncated stem as base.
    let mut counter = 0;
    let final_transcript_path = loop {
        let file_name_part = if counter == 0 {
            format!("{}.json", transcript_filename_stem) // Base name is truncated stem
        } else {
            format!("{}_{}.json", transcript_filename_stem, counter) // Suffix truncated stem
        };
        let path_candidate = import_dir.join(&file_name_part);
        if !path_candidate.exists() {
            break path_candidate;
        }
        counter += 1;
        if counter > 100 { // Safety break
            return Err(CommandError::from(format!("Could not find unique filename for imported transcript (base: '{}') after {} attempts.", transcript_filename_stem, counter)));
        }
    };
    // new_transcript_filename is the final name, e.g., truncated_stem.json or truncated_stem_1.json
    let new_transcript_filename = final_transcript_path.file_name().unwrap().to_string_lossy().to_string();


    let json_content = serde_json::to_string_pretty(&segments)
        .map_err(|e| CommandError::from(format!("Failed to serialize segments to JSON: {}", e)))?;
    fs::write(&final_transcript_path, json_content)
        .map_err(|e| CommandError::from(format!("Failed to save transcript JSON to {}: {}", final_transcript_path.display(), e)))?;
    info!("[import_word_transcript] Saved standalone transcript to: {}", final_transcript_path.display());

    // --- Save metadata to DB ---
    let file_metadata_for_db = FileMetadata {
        file_name: new_transcript_filename.clone(), // Use final (potentially suffixed) truncated filename
        file_path: final_transcript_path.to_string_lossy().into_owned(),
        last_modified: Utc::now().to_rfc3339(),
        title: String::new(),
        description: String::new(),
        summary: String::new(),
        duration_seconds: None,
        width: None,
        height: None,
        frame_rate: None,
        bit_rate: None,
        audio_codec: None,
        video_codec: None,
        created_at: Some(Utc::now().to_rfc3339()),
                original_import_path: None,
                speaker_names: None,
                waveform_data: None,
    };

    let asset_relative_path_for_db = final_transcript_path
        .strip_prefix(project_base_dir)?
        .to_string_lossy()
        .replace("\\", "/");

    let asset_type = "imported_transcript"; // Define asset type

    // Read project_uuid from XML
    // project_xml_path is already a PathBuf from the function arguments
    let project_xml_content_for_uuid = fs::read_to_string(&project_xml_path)
        .map_err(|e| CommandError::Io(format!("Failed to read project XML for UUID: {}", e)))?;
    let project_data_for_uuid: ProjectXml = quick_xml::de::from_str(&project_xml_content_for_uuid)
        .map_err(|e| CommandError::XmlDeserialization(format!("Failed to parse project XML for UUID: {}", e)))?;

    let project_id_for_db = project_data_for_uuid.project_uuid;
    if project_id_for_db.is_empty() {
        error!("[import_word_transcript] Project UUID is empty in XML file: {}. Cannot save asset metadata without project_id.", project_xml_path.display());
        return Err(CommandError::Message(format!("Project ID (UUID) is missing in the project file ({}). Asset metadata cannot be saved.", project_xml_path.display())));
    }

    if let Err(e) = db_handler::save_asset_metadata(
        &project_id_for_db, // Pass project_id
        &file_metadata_for_db,
        &asset_relative_path_for_db, // DB key uses final (potentially suffixed) truncated name based path
        asset_type,
        None, // custom_fields_json is None on initial import
    ) {
        error!("Failed to save transcript metadata to DB for {}: {} (project_id: {})", asset_relative_path_for_db, e, project_id_for_db);
        // Consider cleaning up the created JSON file if DB save fails
        return Err(CommandError::from(format!("Failed to save transcript metadata to DB: {}", e)));
    }
    info!("[import_word_transcript] Saved transcript metadata to DB for: {}", asset_relative_path_for_db);
    // --- End of DB metadata saving ---

    // Update Project XML
    let mut project_data: ProjectXml = quick_xml::de::from_str(&fs::read_to_string(&project_xml_path)?)?;
    
    // The relative_transcript_path_for_xml is the same as asset_relative_path_for_db used above
    let relative_transcript_path_for_xml = asset_relative_path_for_db; // Path uses final (potentially suffixed) truncated name

    let new_imported_transcript_entry = ImportedTranscriptEntryXml {
        name: new_transcript_filename.clone(), // XML name is the final (potentially suffixed) truncated filename
        relative_path: relative_transcript_path_for_xml.clone(),
    };

    if !project_data.imported_transcript_files.files.iter().any(|t| t.relative_path == new_imported_transcript_entry.relative_path) {
        project_data.imported_transcript_files.files.push(new_imported_transcript_entry);
        project_data.imported_transcript_files.files.sort_by(|a, b| a.name.cmp(&b.name));
        info!("[import_word_transcript] Added new imported transcript entry to XML project data.");
    } else {
        warn!("[import_word_transcript] Standalone transcript with relative path {} already exists in XML. Not adding duplicate.", new_imported_transcript_entry.relative_path);
    }

    // The block for adding .metadata.json to project_data.document_metadata_files.files is REMOVED.
    // Metadata is now in the database and not tracked as a separate file entry in the XML for this type.

    save_project_xml(&project_xml_path, &project_data)?;
    info!("[import_word_transcript] Project XML updated (imported transcript entry only).");

    Ok(final_transcript_path.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    // PathBuf is already imported via super::* if PathBuf is used in super
    use tempfile::tempdir;
    // DocumentMetadataEntryXml and StandardAssetMetadata are no longer directly used here
    use crate::projectview::shared_types::{ProjectXml, ImportedTranscriptEntryXml, FileMetadata, TranscriptSegment};
    use crate::projectview::db_handler; // For direct db interactions in test
    use chrono::Utc;
    use serde_json; // For serializing segments in test setup
    use rusqlite::{Connection, OptionalExtension}; // For in-memory DB

    #[test]
    fn test_metadata_saved_to_db_and_xml_updated_correctly() -> Result<(), Box<dyn std::error::Error>> {
        // 1. Setup
        let temp_dir = tempdir()?;
        let project_base_dir = temp_dir.path();

        let docx_filename_stem = "test_transcript_from_docx_db";

        let transcript_specific_dir = project_base_dir
            .join(HARVEY_FILES_DIR)
            .join(TRANSCRIPTS_DIR)
            .join(docx_filename_stem);
        fs::create_dir_all(&transcript_specific_dir)?;

        let project_xml_path = project_base_dir.join("project.xml");
        let initial_project_data = ProjectXml {
            name: "Test Project DB".to_string(),
            project_uuid: "test_uuid_db_xml".to_string(),
            media_files: Default::default(),
            document_files: Default::default(),
            table_files: Default::default(),
            image_files: Default::default(),
            imported_transcript_files: Default::default(),
            document_metadata_files: Default::default(),
        };
        let xml_string = quick_xml::se::to_string(&initial_project_data)?;
        fs::write(&project_xml_path, xml_string)?;

        let new_transcript_filename = format!("{}.json", docx_filename_stem);
        let final_transcript_path = transcript_specific_dir.join(&new_transcript_filename);

        let segments = vec![TranscriptSegment { start_time: 0.0, end_time: 1.0, speaker: "S1".to_string(), text: "Test Content".to_string() }];
        let json_content_segments = serde_json::to_string_pretty(&segments)?;
        fs::write(&final_transcript_path, json_content_segments)?;
        assert!(final_transcript_path.exists(), "Transcript file should be created for test setup");

        // --- Test DB Setup (In-Memory) ---
        let conn_test_db = Connection::open_in_memory()?;
        // Manually run the DDL for asset_metadata table and trigger for the test connection
        conn_test_db.execute_batch(
            "CREATE TABLE IF NOT EXISTS asset_metadata (
                asset_relative_path TEXT NOT NULL,
                project_id TEXT NOT NULL,
                file_name TEXT NOT NULL,
                file_path TEXT NOT NULL,
                last_modified TEXT NOT NULL,
                title TEXT,
                description TEXT,
                summary TEXT,
                duration_seconds REAL,
                width INTEGER,
                height INTEGER,
                frame_rate REAL,
                bit_rate INTEGER,
                audio_codec TEXT,
                video_codec TEXT,
                creation_time TEXT,
                asset_type TEXT NOT NULL,
                custom_fields_json TEXT,
                original_import_path TEXT,
                speaker_names_json TEXT,
                waveform_data BLOB,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                PRIMARY KEY (project_id, asset_relative_path)
            );
            CREATE TRIGGER IF NOT EXISTS update_asset_metadata_updated_at
            AFTER UPDATE ON asset_metadata FOR EACH ROW BEGIN
                UPDATE asset_metadata SET updated_at = CURRENT_TIMESTAMP WHERE project_id = OLD.project_id AND asset_relative_path = OLD.asset_relative_path;
            END;",
        )?;

        // --- Simulate core logic of import_word_transcript related to metadata and XML ---
        // This part replicates the logic that would be in import_word_transcript

        // 1. Create FileMetadata for DB
        let file_metadata_for_db_obj = FileMetadata {
            file_name: new_transcript_filename.clone(),
            file_path: final_transcript_path.to_string_lossy().into_owned(),
            last_modified: Utc::now().to_rfc3339(),
            title: String::new(), description: String::new(), summary: String::new(),
            duration_seconds: None, width: None, height: None, frame_rate: None,
            bit_rate: None, audio_codec: None, video_codec: None, created_at: None,
            original_import_path: None,
            speaker_names: None,
            waveform_data: None,
        };

        let asset_relative_path_for_db_str = final_transcript_path
            .strip_prefix(project_base_dir)?
            .to_string_lossy()
            .replace("\\", "/");
        let asset_type_str = "imported_transcript";

        // 2. Simulate saving metadata to DB (using the test in-memory connection)
        // Replicating db_handler::save_asset_metadata's core SQL for the test:
        {
            let custom_fields_json_val: Option<&str> = None;
            let sql_insert = "
                INSERT INTO asset_metadata (
                    project_id, asset_relative_path, file_name, file_path, last_modified, title,
                    description, summary, duration_seconds, width, height, frame_rate,
                    bit_rate, audio_codec, video_codec, creation_time, asset_type, custom_fields_json,
                    original_import_path, speaker_names_json, waveform_data
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)
                ON CONFLICT(project_id, asset_relative_path) DO UPDATE SET
                    file_name = excluded.file_name, file_path = excluded.file_path, last_modified = excluded.last_modified,
                    title = excluded.title, description = excluded.description, summary = excluded.summary,
                    duration_seconds = excluded.duration_seconds, width = excluded.width, height = excluded.height,
                    frame_rate = excluded.frame_rate, bit_rate = excluded.bit_rate, audio_codec = excluded.audio_codec,
                    video_codec = excluded.video_codec, creation_time = excluded.creation_time,
                    asset_type = excluded.asset_type, custom_fields_json = excluded.custom_fields_json,
                    original_import_path = excluded.original_import_path,
                    speaker_names_json = excluded.speaker_names_json,
                    waveform_data = excluded.waveform_data,
                    updated_at = CURRENT_TIMESTAMP;
            ";
             conn_test_db.execute(
                sql_insert,
                rusqlite::params![
                    "test_uuid_db_xml",
                    asset_relative_path_for_db_str,
                    file_metadata_for_db_obj.file_name,
                    file_metadata_for_db_obj.file_path,
                    file_metadata_for_db_obj.last_modified,
                    &file_metadata_for_db_obj.title,
                    &file_metadata_for_db_obj.description,
                    &file_metadata_for_db_obj.summary,
                    db_handler::to_sql_optional(file_metadata_for_db_obj.duration_seconds),
                    db_handler::to_sql_optional(file_metadata_for_db_obj.width),
                    db_handler::to_sql_optional(file_metadata_for_db_obj.height),
                    db_handler::to_sql_optional(file_metadata_for_db_obj.frame_rate),
                    db_handler::to_sql_optional(file_metadata_for_db_obj.bit_rate),
                    db_handler::to_sql_optional_str(file_metadata_for_db_obj.audio_codec.as_deref()),
                    db_handler::to_sql_optional_str(file_metadata_for_db_obj.video_codec.as_deref()),
                    db_handler::to_sql_optional_str(file_metadata_for_db_obj.created_at.as_deref()),
                    asset_type_str,
                    db_handler::to_sql_optional_str(custom_fields_json_val),
                    db_handler::to_sql_optional_str(file_metadata_for_db_obj.original_import_path.as_deref()),
                    db_handler::to_sql_optional_str(None), // speaker_names_json
                    db_handler::to_sql_optional_blob(file_metadata_for_db_obj.waveform_data.as_deref()),
                ],
            )?;
        }

        // 3. Simulate updating Project XML
        let mut current_project_data: ProjectXml = quick_xml::de::from_str(&fs::read_to_string(&project_xml_path)?)?;
        let new_imported_transcript_entry_obj = ImportedTranscriptEntryXml {
            name: new_transcript_filename.clone(),
            relative_path: asset_relative_path_for_db_str.clone(),
        };
        if !current_project_data.imported_transcript_files.files.iter().any(|t| t.relative_path == new_imported_transcript_entry_obj.relative_path) {
            current_project_data.imported_transcript_files.files.push(new_imported_transcript_entry_obj.clone());
            current_project_data.imported_transcript_files.files.sort_by(|a, b| a.name.cmp(&b.name));
        }
        // The part for adding to project_data.document_metadata_files is intentionally removed.
        save_project_xml(&project_xml_path, &current_project_data)?;

        // --- Assertions ---
        // Verify DB Data by loading it
        let loaded_db_meta_opt: Option<db_handler::FileMetadataWithCustomFieldsFromDb> = {
             let mut stmt_load = conn_test_db.prepare("
                SELECT file_name, file_path, last_modified, title, description, summary,
                       duration_seconds, width, height, frame_rate, bit_rate, audio_codec, video_codec,
                       creation_time, custom_fields_json, asset_type, original_import_path, speaker_names_json, waveform_data
                FROM asset_metadata WHERE project_id = ?1 AND asset_relative_path = ?2
            ")?;
            stmt_load.query_row(rusqlite::params!["test_uuid_db_xml", asset_relative_path_for_db_str], |row| {
                Ok(db_handler::FileMetadataWithCustomFieldsFromDb {
                    file_name: row.get(0)?, file_path: row.get(1)?, last_modified: row.get(2)?,
                    title: row.get(3)?, description: row.get(4)?, summary: row.get(5)?,
                    duration_seconds: row.get(6)?, width: row.get(7)?, height: row.get(8)?,
                    frame_rate: row.get(9)?, bit_rate: row.get(10)?, audio_codec: row.get(11)?,
                    video_codec: row.get(12)?, creation_time: row.get(13)?,
                    custom_fields_json: row.get(14)?, asset_type: row.get(15)?,
                    original_import_path: row.get(16)?, speaker_names_json: row.get(17)?, waveform_data: row.get(18)?,
                })
            }).optional()?
        };

        assert!(loaded_db_meta_opt.is_some(), "Metadata should be found in DB");
        if let Some(loaded_db_meta_val) = loaded_db_meta_opt {
            assert_eq!(loaded_db_meta_val.file_name, file_metadata_for_db_obj.file_name);
            assert_eq!(loaded_db_meta_val.file_path, file_metadata_for_db_obj.file_path);
            assert_eq!(loaded_db_meta_val.title.unwrap_or_default(), file_metadata_for_db_obj.title);
            assert_eq!(loaded_db_meta_val.asset_type, asset_type_str);
            assert!(loaded_db_meta_val.custom_fields_json.is_none());
        }

        // Verify XML Data
        let updated_project_data_from_xml_check: ProjectXml = quick_xml::de::from_str(&fs::read_to_string(&project_xml_path)?)?;
        assert_eq!(updated_project_data_from_xml_check.imported_transcript_files.files.len(), 1);
        assert_eq!(updated_project_data_from_xml_check.imported_transcript_files.files[0].name, new_imported_transcript_entry_obj.name);
        assert!(updated_project_data_from_xml_check.document_metadata_files.files.is_empty(), "Document metadata files list in XML should be empty regarding this transcript.");

        Ok(())
    }
}
