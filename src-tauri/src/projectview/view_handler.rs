use crate::welcome::config::CommandError;
use crate::projectview::db_handler::{self, get_db_path};
use crate::projectview::shared_types::FileMetadata;
use crate::projectview::table_handler;
use crate::projectview::transcription_commands::create_lexical_paragraph_json_value;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use log::{info, debug, error};
use std::path::PathBuf;
use std::fs;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ViewConfig {
    pub id: Option<i64>,
    pub project_id: String,
    pub table_path: String,
    pub view_name: String,
    pub view_type: String,
    pub config_json: String,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

pub fn save_table_view(
    project_id: &str,
    table_path: &str,
    view_name: &str,
    view_type: &str,
    config_json: &str,
) -> Result<ViewConfig, CommandError> {
    let db_path = get_db_path()?;
    let conn = Connection::open(db_path)?;

    info!("[DB] Saving table view '{}' for table '{}' in project '{}'", view_name, table_path, project_id);

    conn.execute(
        "INSERT INTO table_views (project_id, table_path, view_name, view_type, config_json)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(project_id, table_path, view_name) DO UPDATE SET
         view_type = excluded.view_type,
         config_json = excluded.config_json,
         updated_at = CURRENT_TIMESTAMP",
        params![project_id, table_path, view_name, view_type, config_json],
    )?;

    let mut stmt = conn.prepare(
        "SELECT id, project_id, table_path, view_name, view_type, config_json, created_at, updated_at
         FROM table_views
         WHERE project_id = ?1 AND table_path = ?2 AND view_name = ?3"
    )?;

    let view = stmt.query_row(params![project_id, table_path, view_name], |row| {
        Ok(ViewConfig {
            id: row.get(0)?,
            project_id: row.get(1)?,
            table_path: row.get(2)?,
            view_name: row.get(3)?,
            view_type: row.get(4)?,
            config_json: row.get(5)?,
            created_at: row.get(6)?,
            updated_at: row.get(7)?,
        })
    })?;

    Ok(view)
}

pub async fn generate_survey_documents(
    project_id: &str,
    table_path: &str,
    view_name: &str,
    config_json: &str,
    project_xml_path_str: &str,
) -> Result<Vec<String>, CommandError> {
    info!("[Survey] Generating survey documents for project_id={}, table_path={}, view_name={}", project_id, table_path, view_name);

    let config: Value = serde_json::from_str(config_json)
        .map_err(|e| CommandError::Io(format!("Failed to parse config_json: {}", e)))?;

    let survey_group_by_type = config.get("surveyGroupByType").and_then(|v: &Value| v.as_str()).unwrap_or("Participants");
    let survey_unique_identifier_field = config.get("surveyUniqueIdentifierField").and_then(|v: &Value| v.as_str()).unwrap_or("");

    let project_xml_path = PathBuf::from(project_xml_path_str);
    let project_base_dir = project_xml_path.parent().ok_or_else(|| CommandError::Path("Could not get project base directory.".to_string()))?;

    // Determine the base attachments directory for this table
    let table_path_buf = PathBuf::from(table_path);
    let table_dir = table_path_buf.parent().ok_or_else(|| CommandError::Path("Invalid table path".to_string()))?;

    let rel_base_attachments_dir = table_dir.join("attachments");
    let abs_base_attachments_dir = project_base_dir.join(&rel_base_attachments_dir);

    if !abs_base_attachments_dir.exists() {
        fs::create_dir_all(&abs_base_attachments_dir)
            .map_err(|e| CommandError::Io(format!("Failed to create attachments dir: {}", e)))?;
    }

    // Load table data
    let table_data: Value = table_handler::load_table_data(table_path.to_string()).await?;
    let rows = table_data.as_array().ok_or_else(|| CommandError::Io("Table data is not an array".to_string()))?;

    let mut generated_files = Vec::new();

    if survey_group_by_type == "Participants" {
        let survey_participant_included_fields = config.get("surveyParticipantIncludedFields")
            .and_then(|v: &Value| v.as_array())
            .map(|arr| arr.iter().filter_map(|v: &Value| v.as_str()).collect::<Vec<_>>())
            .unwrap_or_default();

        let target_dir_name = format!("{}_participants", view_name);
        let rel_target_dir = rel_base_attachments_dir.join(&target_dir_name);
        let abs_target_dir = project_base_dir.join(&rel_target_dir);

        if !abs_target_dir.exists() {
            fs::create_dir_all(&abs_target_dir)
                .map_err(|e| CommandError::Io(format!("Failed to create target dir: {}", e)))?;
        }

        for (index, row_val) in rows.iter().enumerate() {
            let row = row_val.as_object().unwrap();
            let participant_id = if !survey_unique_identifier_field.is_empty() {
                row.get(survey_unique_identifier_field)
                    .and_then(|v: &Value| v.as_str())
                    .unwrap_or(&format!("Participant_{}", index + 1))
                    .to_string()
            } else {
                format!("Participant_{}", index + 1)
            };

            // Sanitize filename
            let safe_filename = participant_id.replace(&['/', '\\', ':', '*', '?', '"', '<', '>', '|'][..], "_");
            let rel_file_path = rel_target_dir.join(format!("{}.json", safe_filename));
            let abs_file_path = project_base_dir.join(&rel_file_path);

            let mut lexical_children = Vec::new();

            for field in &survey_participant_included_fields {
                let value_str = row.get(*field)
                    .map(|v: &Value| {
                        if v.is_string() { v.as_str().unwrap().to_string() }
                        else if v.is_null() { "".to_string() }
                        else { v.to_string() }
                    })
                    .unwrap_or_default();

                // Bold Field Name
                lexical_children.push(json!({
                    "type": "paragraph",
                    "version": 1,
                    "children": [{
                        "detail": 0,
                        "format": 1, // 1 is Bold
                        "mode": "normal",
                        "style": "",
                        "text": field,
                        "type": "extended-text",
                        "version": 1,
                        "highlightId": null
                    }],
                    "direction": "ltr",
                    "format": "",
                    "indent": 0
                }));

                // Value paragraph
                lexical_children.push(create_lexical_paragraph_json_value(&value_str));
            }

            let doc_json = json!({
                "root": {
                    "children": lexical_children,
                    "direction": "ltr",
                    "format": "",
                    "indent": 0,
                    "type": "root",
                    "version": 1
                }
            });

            fs::write(&abs_file_path, serde_json::to_string_pretty(&doc_json).unwrap())
                .map_err(|e| CommandError::Io(format!("Failed to write document {}: {}", abs_file_path.display(), e)))?;

            // Store relative path in DB
            generated_files.push(rel_file_path.to_string_lossy().to_string().replace("\\", "/"));
        }
    } else {
        // "Questions"
        let survey_selected_questions = config.get("surveySelectedQuestions")
            .and_then(|v: &Value| v.as_array())
            .map(|arr| arr.iter().filter_map(|v: &Value| v.as_str()).collect::<Vec<_>>())
            .unwrap_or_default();

        let survey_included_other_fields = config.get("surveyIncludedOtherFields")
            .and_then(|v: &Value| v.as_array())
            .map(|arr| arr.iter().filter_map(|v: &Value| v.as_str()).collect::<Vec<_>>())
            .unwrap_or_default();

        let target_dir_name = format!("{}_questions", view_name);
        let rel_target_dir = rel_base_attachments_dir.join(&target_dir_name);
        let abs_target_dir = project_base_dir.join(&rel_target_dir);

        if !abs_target_dir.exists() {
            fs::create_dir_all(&abs_target_dir)
                .map_err(|e| CommandError::Io(format!("Failed to create target dir: {}", e)))?;
        }

        for question in survey_selected_questions {
            let safe_filename = question.replace(&['/', '\\', ':', '*', '?', '"', '<', '>', '|'][..], "_");
            let rel_file_path = rel_target_dir.join(format!("{}.json", safe_filename));
            let abs_file_path = project_base_dir.join(&rel_file_path);

            let mut lexical_children = Vec::new();

            // Bold Question as Header
            lexical_children.push(json!({
                "type": "heading",
                "tag": "h2",
                "version": 1,
                "children": [{
                    "detail": 0,
                    "format": 1,
                    "mode": "normal",
                    "style": "",
                    "text": question,
                    "type": "extended-text",
                    "version": 1,
                    "highlightId": null
                }],
                "direction": "ltr",
                "format": "",
                "indent": 0
            }));

            for (index, row_val) in rows.iter().enumerate() {
                let row = row_val.as_object().unwrap();
                let participant_id = if !survey_unique_identifier_field.is_empty() {
                    row.get(survey_unique_identifier_field)
                        .and_then(|v: &Value| v.as_str())
                        .unwrap_or(&format!("Participant_{}", index + 1))
                        .to_string()
                } else {
                    format!("Participant_{}", index + 1)
                };

                // Add participant ID as bold
                lexical_children.push(json!({
                    "type": "paragraph",
                    "version": 1,
                    "children": [{
                        "detail": 0,
                        "format": 1,
                        "mode": "normal",
                        "style": "",
                        "text": participant_id,
                        "type": "extended-text",
                        "version": 1,
                        "highlightId": null
                    }],
                    "direction": "ltr",
                    "format": "",
                    "indent": 0
                }));

                // Add other included fields if any
                for other_field in &survey_included_other_fields {
                    let other_val = row.get(*other_field)
                        .map(|v: &Value| {
                            if v.is_string() { v.as_str().unwrap().to_string() }
                            else if v.is_null() { "".to_string() }
                            else { v.to_string() }
                        })
                        .unwrap_or_default();

                    lexical_children.push(create_lexical_paragraph_json_value(&format!("{}: {}", other_field, other_val)));
                }

                // Add response
                let response_val = row.get(question)
                    .map(|v: &Value| {
                        if v.is_string() { v.as_str().unwrap().to_string() }
                        else if v.is_null() { "".to_string() }
                        else { v.to_string() }
                    })
                    .unwrap_or_default();

                lexical_children.push(create_lexical_paragraph_json_value(&response_val));

                // Add a blank separator paragraph for readability
                lexical_children.push(create_lexical_paragraph_json_value(""));
            }

            let doc_json = json!({
                "root": {
                    "children": lexical_children,
                    "direction": "ltr",
                    "format": "",
                    "indent": 0,
                    "type": "root",
                    "version": 1
                }
            });

            fs::write(&abs_file_path, serde_json::to_string_pretty(&doc_json).unwrap())
                .map_err(|e| CommandError::Io(format!("Failed to write document {}: {}", abs_file_path.display(), e)))?;

            generated_files.push(rel_file_path.to_string_lossy().to_string().replace("\\", "/"));
        }
    }

    // Now update asset_metadata to include these as attachments for the table
    if let Ok(Some(metadata_from_db)) = db_handler::load_asset_metadata(project_id, table_path) {
        let mut custom_fields: Vec<serde_json::Value> = metadata_from_db.custom_fields_json
            .as_deref()
            .and_then(|json| serde_json::from_str(json).ok())
            .unwrap_or_else(Vec::new);

        let mut attachments: Vec<String> = custom_fields.iter()
            .find(|f: &&Value| f.get("key").and_then(|k: &Value| k.as_str()) == Some("attachments"))
            .and_then(|f: &Value| f.get("value").and_then(|v: &Value| v.as_str()))
            .and_then(|v| serde_json::from_str(v).ok())
            .unwrap_or_else(Vec::new);

        for new_file in &generated_files {
            if !attachments.contains(new_file) {
                attachments.push(new_file.clone());
            }

            let file_name = PathBuf::from(new_file).file_name().unwrap().to_string_lossy().to_string();
            // Register each attachment individually
            let attachment_metadata = FileMetadata {
                file_name: file_name,
                file_path: new_file.clone(),
                last_modified: chrono::Utc::now().to_rfc3339(),
                file_type: "document".to_string(), // Lexical docs are 'document'
                ..Default::default()
            };

            let _ = db_handler::save_asset_metadata(
                project_id,
                &attachment_metadata,
                new_file,
                "attachment",
                None
            );
        }

        let attachments_json_string = json!(attachments).to_string();

        if let Some(existing_field) = custom_fields.iter_mut().find(|f: &&mut Value| f.get("key").and_then(|k: &Value| k.as_str()) == Some("attachments")) {
            if let Some(obj) = existing_field.as_object_mut() {
                obj.insert("value".to_string(), json!(attachments_json_string));
            }
        } else {
            let new_field = json!({
                "key": "attachments",
                "value": attachments_json_string
            });
            custom_fields.push(new_field);
        }

        let updated_custom_fields_json_str = serde_json::to_string(&custom_fields).unwrap_or_else(|_| "[]".to_string());

        let file_metadata = FileMetadata {
            file_name: metadata_from_db.file_name,
            file_path: metadata_from_db.file_path,
            last_modified: chrono::Utc::now().to_rfc3339(),
            title: metadata_from_db.title.unwrap_or_default(),
            description: metadata_from_db.description.unwrap_or_default(),
            summary: metadata_from_db.summary.unwrap_or_default(),
            duration_seconds: metadata_from_db.duration_seconds,
            width: metadata_from_db.width,
            height: metadata_from_db.height,
            frame_rate: metadata_from_db.frame_rate,
            bit_rate: metadata_from_db.bit_rate,
            audio_codec: metadata_from_db.audio_codec,
            video_codec: metadata_from_db.video_codec,
            created_at: metadata_from_db.creation_time,
            original_import_path: metadata_from_db.original_import_path,
            speaker_names: metadata_from_db.speaker_names_json.and_then(|s| serde_json::from_str(&s).ok()),
            waveform_data: metadata_from_db.waveform_data,
            language_code: metadata_from_db.language_code,
            properties: metadata_from_db.properties,
            file_type: metadata_from_db.file_type.unwrap_or_default(),
        };

        let _ = db_handler::save_asset_metadata(
            project_id,
            &file_metadata,
            table_path,
            &metadata_from_db.asset_type,
            Some(&updated_custom_fields_json_str)
        );
    }

    Ok(generated_files)
}

pub fn load_table_views(
    project_id: &str,
    table_path: &str,
) -> Result<Vec<ViewConfig>, CommandError> {
    let db_path = get_db_path()?;
    let conn = Connection::open(db_path)?;

    debug!("[DB] Loading table views for table '{}' in project '{}'", table_path, project_id);

    let mut stmt = conn.prepare(
        "SELECT id, project_id, table_path, view_name, view_type, config_json, created_at, updated_at
         FROM table_views
         WHERE project_id = ?1 AND table_path = ?2
         ORDER BY updated_at DESC"
    )?;

    let view_iter = stmt.query_map(params![project_id, table_path], |row| {
        Ok(ViewConfig {
            id: row.get(0)?,
            project_id: row.get(1)?,
            table_path: row.get(2)?,
            view_name: row.get(3)?,
            view_type: row.get(4)?,
            config_json: row.get(5)?,
            created_at: row.get(6)?,
            updated_at: row.get(7)?,
        })
    })?;

    let mut views = Vec::new();
    for view in view_iter {
        views.push(view?);
    }

    Ok(views)
}

pub fn delete_table_view(
    project_id: &str,
    table_path: &str,
    view_name: &str,
    project_xml_path_str: &str,
) -> Result<(), CommandError> {
    let db_path = get_db_path()?;
    let conn = Connection::open(db_path)?;

    info!("[DB] Deleting table view '{}' for table '{}' in project '{}'", view_name, table_path, project_id);

    // Fetch the view to see if it's a survey view that needs file cleanup
    let mut stmt = conn.prepare("SELECT view_type, config_json FROM table_views WHERE project_id = ?1 AND table_path = ?2 AND view_name = ?3")?;
    let view_data = stmt.query_row(params![project_id, table_path, view_name], |row| {
        let view_type: String = row.get(0)?;
        let config_json: String = row.get(1)?;
        Ok((view_type, config_json))
    }).optional()?;

    if let Some((view_type, config_json)) = view_data {
        if view_type == "survey" {
            info!("Deleting files for survey view: {}", view_name);
            let config: Value = serde_json::from_str(&config_json).unwrap_or(json!({}));
            let group_by_type = config.get("surveyGroupByType").and_then(|v: &Value| v.as_str()).unwrap_or("Participants");

            let project_xml_path = PathBuf::from(project_xml_path_str);
            let project_base_dir = project_xml_path.parent().ok_or_else(|| CommandError::Path("Could not get project base directory.".to_string()))?;
            let table_path_buf = PathBuf::from(table_path);

            if let Some(table_dir) = table_path_buf.parent() {
                let target_dir_name = if group_by_type == "Participants" {
                    format!("{}_participants", view_name)
                } else {
                    format!("{}_questions", view_name)
                };

                let rel_target_dir = table_dir.join("attachments").join(&target_dir_name);
                let abs_target_dir = project_base_dir.join(&rel_target_dir);

                if abs_target_dir.exists() {
                    // Collect all JSON files in the directory to remove them from DB attachments
                    // The DB expects the relative paths, so we rebuild them relative to the project
                    let mut files_to_remove = Vec::new();
                    if let Ok(entries) = fs::read_dir(&abs_target_dir) {
                        for entry in entries.filter_map(Result::ok) {
                            if let Some(file_name) = entry.file_name().to_str() {
                                let rel_file_path = rel_target_dir.join(file_name).to_string_lossy().to_string().replace("\\", "/");
                                files_to_remove.push(rel_file_path);
                            }
                        }
                    }

                    // Delete from disk using absolute path
                    if let Err(e) = fs::remove_dir_all(&abs_target_dir) {
                        error!("Failed to remove survey directory {}: {}", abs_target_dir.display(), e);
                    }

                    // Remove from DB asset_metadata attachments
                    if let Ok(Some(metadata_from_db)) = db_handler::load_asset_metadata(project_id, table_path) {
                        let mut custom_fields: Vec<serde_json::Value> = metadata_from_db.custom_fields_json
                            .as_deref()
                            .and_then(|json| serde_json::from_str(json).ok())
                            .unwrap_or_else(Vec::new);

                        let mut attachments: Vec<String> = custom_fields.iter()
                            .find(|f: &&Value| f.get("key").and_then(|k: &Value| k.as_str()) == Some("attachments"))
                            .and_then(|f: &Value| f.get("value").and_then(|v: &Value| v.as_str()))
                            .and_then(|v| serde_json::from_str(v).ok())
                            .unwrap_or_else(Vec::new);

                        attachments.retain(|a| !files_to_remove.contains(a));

                        // Also remove individual attachment rows
                        for file_to_remove in files_to_remove {
                            let _ = conn.execute(
                                "DELETE FROM asset_metadata WHERE project_id = ?1 AND asset_relative_path = ?2",
                                params![project_id, file_to_remove],
                            );
                        }

                        let attachments_json_string = json!(attachments).to_string();

                        if let Some(existing_field) = custom_fields.iter_mut().find(|f: &&mut Value| f.get("key").and_then(|k: &Value| k.as_str()) == Some("attachments")) {
                            if let Some(obj) = existing_field.as_object_mut() {
                                obj.insert("value".to_string(), json!(attachments_json_string));
                            }
                        }

                        let updated_custom_fields_json_str = serde_json::to_string(&custom_fields).unwrap_or_else(|_| "[]".to_string());

                        // We must manually execute the update here to avoid borrowing issues or just update the field
                        let _ = conn.execute(
                            "UPDATE asset_metadata SET custom_fields_json = ?1 WHERE project_id = ?2 AND asset_relative_path = ?3",
                            params![updated_custom_fields_json_str, project_id, table_path],
                        );
                    }
                }
            }
        }
    }

    conn.execute(
        "DELETE FROM table_views WHERE project_id = ?1 AND table_path = ?2 AND view_name = ?3",
        params![project_id, table_path, view_name],
    )?;

    Ok(())
}

pub fn delete_all_table_views_for_table(
    project_id: &str,
    table_path: &str,
) -> Result<(), CommandError> {
    let db_path = get_db_path()?;
    let conn = Connection::open(db_path)?;

    info!("[DB] Deleting all table views for table '{}' in project '{}'", table_path, project_id);

    conn.execute(
        "DELETE FROM table_views WHERE project_id = ?1 AND table_path = ?2",
        params![project_id, table_path],
    )?;

    Ok(())
}
