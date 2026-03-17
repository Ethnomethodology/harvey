use crate::welcome::config::CommandError;
use crate::projectview::db_handler::get_db_path;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use log::{info, debug};

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
) -> Result<(), CommandError> {
    let db_path = get_db_path()?;
    let conn = Connection::open(db_path)?;

    info!("[DB] Deleting table view '{}' for table '{}' in project '{}'", view_name, table_path, project_id);

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
