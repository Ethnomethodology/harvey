use crate::welcome::config::CommandError;
use crate::projectview::db_handler::{self, get_db_path};
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
