// src-tauri/src/projectview/transcription_handler.rs
use super::shared_types::{
    TranscriptSegment, ProjectXml, ImportedTranscriptEntryXml,
    StandardAssetMetadata, FileMetadata, DocumentMetadataEntryXml,
    HARVEY_FILES_DIR, DOCS_DIR, TEMP_SUBDIR_DOCS, TRANSCRIPTS_DIR,
};
use super::shared_utils::*;
use crate::welcome::config::CommandError;
use regex::Regex;
use std::{
    fs,
    path::{Path, PathBuf}, // Path added
    // time::{SystemTime, UNIX_EPOCH}, // Removed as timestamp is no longer in filename
};
use chrono::Utc; // Added for timestamping metadata
use tauri::AppHandle;
use tauri_plugin_shell::ShellExt;
use uuid::Uuid; // For temp file uniqueness
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
pub async fn import_word_transcript(
    app_handle: AppHandle,
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
    let docx_filename_stem = source_docx_path.file_stem().and_then(|s| s.to_str()).unwrap_or("imported_transcript").to_string();

    let temp_html_dir = project_base_dir.join(HARVEY_FILES_DIR).join(DOCS_DIR).join(TEMP_SUBDIR_DOCS);
    fs::create_dir_all(&temp_html_dir).map_err(|e| CommandError::from(format!("Failed to create temp dir for HTML: {}", e)))?;
    let unique_id = Uuid::new_v4().to_string(); 
    let temp_html_filename = format!("temp_transcript_html_{}_{}.html", docx_filename_stem, unique_id);
    let temp_html_path = temp_html_dir.join(&temp_html_filename);

    let pandoc_args = vec![
        source_docx_path.to_string_lossy().to_string(),
        "-f".to_string(), "docx".to_string(),
        "-t".to_string(), "html".to_string(),
        "--standalone".to_string(),
        "-o".to_string(), temp_html_path.to_string_lossy().to_string(),
    ];

    info!("[import_word_transcript] Pandoc CMD: pandoc {}", pandoc_args.join(" "));
    let (mut rx, _child) = app_handle.shell().sidecar("pandoc")?.args(&pandoc_args).spawn()
        .map_err(|e| CommandError::from(format!("Failed to spawn Pandoc: {}", e)))?;

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

    // Create a dedicated subdirectory for this imported transcript
    let import_dir = target_standalone_transcripts_dir.join(&docx_filename_stem);
    fs::create_dir_all(&import_dir)
        .map_err(|e| CommandError::from(format!(
            "Failed to create imported transcript dir {}: {}", 
            import_dir.display(), e
        )))?;

    let mut counter = 0;
    let final_transcript_path = loop {
        let file_name_part = if counter == 0 {
            format!("{}.json", docx_filename_stem)
        } else {
            format!("{}_{}.json", docx_filename_stem, counter)
        };
        let path_candidate = import_dir.join(&file_name_part);
        if !path_candidate.exists() {
            break path_candidate;
        }
        counter += 1;
        if counter > 100 { 
            return Err(CommandError::from(format!("Could not find unique filename for imported transcript after {} attempts.", counter)));
        }
    };
    let new_transcript_filename = final_transcript_path.file_name().unwrap().to_string_lossy().to_string();


    let json_content = serde_json::to_string_pretty(&segments)
        .map_err(|e| CommandError::from(format!("Failed to serialize segments to JSON: {}", e)))?;
    fs::write(&final_transcript_path, json_content)
        .map_err(|e| CommandError::from(format!("Failed to save transcript JSON to {}: {}", final_transcript_path.display(), e)))?;
    info!("[import_word_transcript] Saved standalone transcript to: {}", final_transcript_path.display());

    // Create metadata file
    let final_transcript_path_obj = Path::new(&final_transcript_path); // Use Path for operations
    let transcript_dir = final_transcript_path_obj.parent().ok_or_else(|| CommandError::from("Could not get transcript directory"))?;
    let transcript_filename_stem = final_transcript_path_obj.file_stem().and_then(|s| s.to_str()).ok_or_else(|| CommandError::from("Could not get transcript filename stem"))?;

    let metadata_filename = format!(".{}.metadata.json", transcript_filename_stem);
    let metadata_file_path = transcript_dir.join(&metadata_filename);

    let file_meta = FileMetadata {
        file_name: new_transcript_filename.clone(), // Name of the main JSON transcript file
        file_path: final_transcript_path.to_string_lossy().into_owned(), // Full absolute path to the main JSON transcript file
        last_modified: Utc::now().to_rfc3339(),
        title: String::new(),
        description: String::new(),
        summary: String::new(),
        duration: None,
        width: None,
        height: None,
        file_type: Some("application/json".to_string()), // Assuming transcript is JSON
        source_url: None,
        tags: Vec::new(),
        chapters: Vec::new(),
        scenes: Vec::new(),
        project_defined_metadata_summary_type: None,
    };

    let asset_metadata = StandardAssetMetadata {
        metadata: file_meta,
        highlights: Vec::new(),
    };

    let metadata_json_content = serde_json::to_string_pretty(&asset_metadata)
        .map_err(|e| CommandError::from(format!("Failed to serialize metadata to JSON: {}", e)))?;

    fs::write(&metadata_file_path, metadata_json_content)
        .map_err(|e| CommandError::from(format!("Failed to save metadata JSON to {}: {}", metadata_file_path.display(), e)))?;
    info!("[import_word_transcript] Saved transcript metadata to: {}", metadata_file_path.display());

    // Update Project XML
    let mut project_data: ProjectXml = quick_xml::de::from_str(&fs::read_to_string(&project_xml_path)?)?;
    
    let relative_transcript_path_for_xml = final_transcript_path.strip_prefix(project_base_dir)?.to_string_lossy().replace("\\", "/");

    let new_imported_transcript_entry = ImportedTranscriptEntryXml {
        name: new_transcript_filename.clone(), // This is correct: transcript file name
        relative_path: relative_transcript_path_for_xml.clone(), // This is correct: relative path to transcript json
    };

    if !project_data.imported_transcript_files.files.iter().any(|t| t.relative_path == new_imported_transcript_entry.relative_path) {
        project_data.imported_transcript_files.files.push(new_imported_transcript_entry);
        project_data.imported_transcript_files.files.sort_by(|a, b| a.name.cmp(&b.name));
        info!("[import_word_transcript] Added new imported transcript entry to XML project data.");
    } else {
        warn!("[import_word_transcript] Standalone transcript with relative path {} already exists in XML. Not adding duplicate.", new_imported_transcript_entry.relative_path);
    }

    // Add metadata file entry to project XML
    let metadata_relative_path_for_xml = metadata_file_path.strip_prefix(project_base_dir)?.to_string_lossy().replace("\\", "/");
    let metadata_entry_xml = DocumentMetadataEntryXml {
        name: metadata_file_path.file_name().and_then(|s| s.to_str()).unwrap_or(&metadata_filename).to_string(), // Name of the .metadata.json file
        original_document_relative_path: relative_transcript_path_for_xml, // Relative path to the main transcript json file
        relative_path: metadata_relative_path_for_xml, // Relative path to the .metadata.json file
    };

    if !project_data.document_metadata_files.files.iter().any(|f| f.relative_path == metadata_entry_xml.relative_path) {
        project_data.document_metadata_files.files.push(metadata_entry_xml);
        project_data.document_metadata_files.files.sort_by(|a, b| a.name.cmp(&b.name));
        info!("[import_word_transcript] Added new transcript metadata entry to XML project data: {}", metadata_file_path.display());
    } else {
        warn!("[import_word_transcript] Transcript metadata file with relative path {} already exists in XML. Not adding duplicate.", metadata_entry_xml.relative_path);
    }

    save_project_xml(&project_xml_path, &project_data)?;
    info!("[import_word_transcript] Project XML updated successfully with transcript and its metadata.");

    Ok(final_transcript_path.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::tempdir;
    use crate::projectview::shared_types::{ProjectXml, DocumentMetadataEntryXml, ImportedTranscriptEntryXml, StandardAssetMetadata, FileMetadata, TranscriptSegment};
    use chrono::Utc;
    use serde_json; // For serializing segments in test setup

    #[test]
    fn test_metadata_creation_and_xml_update_for_transcript() -> Result<(), Box<dyn std::error::Error>> {
        // 1. Setup
        let temp_dir = tempdir()?;
        let project_base_dir = temp_dir.path();

        let docx_filename_stem = "test_transcript_from_docx"; // Simulating a stem from a docx

        // Create HARVEY_FILES_DIR/TRANSCRIPTS_DIR/<stem> a similar structure to main code
        let transcript_specific_dir = project_base_dir
            .join(HARVEY_FILES_DIR)
            .join(TRANSCRIPTS_DIR)
            .join(docx_filename_stem);
        fs::create_dir_all(&transcript_specific_dir)?;

        let project_xml_path = project_base_dir.join("project.xml");
        let initial_project_data = ProjectXml {
            project_name: "Test Project".to_string(),
            project_uuid: "test_uuid".to_string(),
            project_root_is_single_file: false,
            video_files: Default::default(),
            audio_files: Default::default(),
            image_files: Default::default(),
            document_files: Default::default(),
            table_files: Default::default(),
            other_files: Default::default(),
            imported_transcript_files: Default::default(),
            document_metadata_files: Default::default(),
            chat_files: Default::default(),
            project_settings: Default::default(),
            saved_searches: Default::default(),
            project_tags: Default::default(),
            project_people: Default::default(),
            project_places: Default::default(),
            project_organizations: Default::default(),
            project_highlights_config: Default::default(),
            project_highlights_filters: Default::default(),
            project_highlights_summary_types: Default::default(),
        };
        let xml_string = quick_xml::se::to_string(&initial_project_data)?;
        fs::write(&project_xml_path, xml_string)?;

        // Paths for the main transcript JSON file
        let new_transcript_filename = format!("{}.json", docx_filename_stem);
        let final_transcript_path = transcript_specific_dir.join(&new_transcript_filename);

        // 2. Simulate pre-requisites: Create a dummy transcript JSON file
        let segments = vec![
            TranscriptSegment {
                start_time: 0.0,
                end_time: 1.0,
                speaker: "Speaker1".to_string(),
                text: "Hello".to_string(),
            },
        ];
        let json_content = serde_json::to_string_pretty(&segments)?;
        fs::write(&final_transcript_path, json_content)?;
        assert!(final_transcript_path.exists(), "Transcript file should be created for test setup");

        // Load initial project data (though we'll re-load it after simulating the core logic)
        let mut project_data: ProjectXml = quick_xml::de::from_str(&fs::read_to_string(&project_xml_path)?)?;

        // --- Start of logic copied and adapted from import_word_transcript ---
        // Metadata File Creation Logic
        let final_transcript_path_obj = Path::new(&final_transcript_path);
        let transcript_dir = final_transcript_path_obj.parent().ok_or_else(|| "Could not get transcript directory in test")?;
        let transcript_filename_stem_str = final_transcript_path_obj.file_stem().and_then(|s| s.to_str()).ok_or_else(|| "Could not get transcript filename stem in test")?;

        let metadata_filename = format!(".{}.metadata.json", transcript_filename_stem_str);
        let metadata_file_path = transcript_dir.join(&metadata_filename);

        let file_meta = FileMetadata {
            file_name: new_transcript_filename.clone(),
            file_path: final_transcript_path.to_string_lossy().into_owned(),
            last_modified: Utc::now().to_rfc3339(),
            title: String::new(),
            description: String::new(),
            summary: String::new(),
            duration: None,
            width: None,
            height: None,
            file_type: Some("application/json".to_string()),
            source_url: None,
            tags: Vec::new(),
            chapters: Vec::new(),
            scenes: Vec::new(),
            project_defined_metadata_summary_type: None,
        };

        let asset_metadata = StandardAssetMetadata {
            metadata: file_meta.clone(), // Clone for later assertion
            highlights: Vec::new(),
        };

        let metadata_json_content = serde_json::to_string_pretty(&asset_metadata)?;
        fs::write(&metadata_file_path, metadata_json_content)?;

        // ProjectXml Update Logic
        let relative_transcript_path_for_xml = final_transcript_path.strip_prefix(project_base_dir)?.to_string_lossy().replace("\\", "/");
        let new_imported_transcript_entry = ImportedTranscriptEntryXml {
            name: new_transcript_filename.clone(),
            relative_path: relative_transcript_path_for_xml.clone(),
        };

        if !project_data.imported_transcript_files.files.iter().any(|t| t.relative_path == new_imported_transcript_entry.relative_path) {
            project_data.imported_transcript_files.files.push(new_imported_transcript_entry.clone()); // Clone for assertion
            project_data.imported_transcript_files.files.sort_by(|a, b| a.name.cmp(&b.name));
        }

        let metadata_relative_path_for_xml = metadata_file_path.strip_prefix(project_base_dir)?.to_string_lossy().replace("\\", "/");
        let metadata_entry_xml = DocumentMetadataEntryXml {
            name: metadata_file_path.file_name().and_then(|s| s.to_str()).unwrap_or(&metadata_filename).to_string(),
            original_document_relative_path: relative_transcript_path_for_xml.clone(),
            relative_path: metadata_relative_path_for_xml.clone(),
        };

        if !project_data.document_metadata_files.files.iter().any(|f| f.relative_path == metadata_entry_xml.relative_path) {
            project_data.document_metadata_files.files.push(metadata_entry_xml.clone()); // Clone for assertion
            project_data.document_metadata_files.files.sort_by(|a, b| a.name.cmp(&b.name));
        }

        save_project_xml(&project_xml_path, &project_data)?; // Use the actual save function
        // --- End of logic copied and adapted ---

        // 3. Assertions
        assert!(metadata_file_path.exists(), "Metadata file should be created");

        let loaded_metadata_content = fs::read_to_string(&metadata_file_path)?;
        let loaded_asset_metadata: StandardAssetMetadata = serde_json::from_str(&loaded_metadata_content)?;

        assert_eq!(loaded_asset_metadata.metadata.file_name, new_transcript_filename, "Metadata file_name mismatch");
        assert_eq!(loaded_asset_metadata.metadata.file_path, final_transcript_path.to_string_lossy(), "Metadata file_path mismatch");
        assert!(loaded_asset_metadata.metadata.title.is_empty(), "Metadata title should be empty");
        assert_eq!(loaded_asset_metadata.metadata.file_type, Some("application/json".to_string()), "Metadata file_type mismatch");


        let updated_project_xml_content = fs::read_to_string(&project_xml_path)?;
        let updated_project_data: ProjectXml = quick_xml::de::from_str(&updated_project_xml_content)?;

        assert_eq!(updated_project_data.imported_transcript_files.files.len(), 1, "Should be one imported transcript file in XML");
        assert_eq!(updated_project_data.imported_transcript_files.files[0].name, new_imported_transcript_entry.name, "Imported transcript name mismatch in XML");
        assert_eq!(updated_project_data.imported_transcript_files.files[0].relative_path, new_imported_transcript_entry.relative_path, "Imported transcript relative_path mismatch in XML");


        assert_eq!(updated_project_data.document_metadata_files.files.len(), 1, "Should be one document metadata file in XML");
        assert_eq!(updated_project_data.document_metadata_files.files[0].name, metadata_entry_xml.name, "Metadata entry name mismatch in XML");
        assert_eq!(updated_project_data.document_metadata_files.files[0].original_document_relative_path, metadata_entry_xml.original_document_relative_path, "Metadata entry original_document_relative_path mismatch in XML");
        assert_eq!(updated_project_data.document_metadata_files.files[0].relative_path, metadata_entry_xml.relative_path, "Metadata entry relative_path mismatch in XML");

        Ok(())
    }
}