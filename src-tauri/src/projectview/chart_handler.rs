// src-tauri/src/projectview/chart_handler.rs
use crate::welcome::config::CommandError;
use crate::projectview::db_handler::get_db_path;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use log::{info, debug, error};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChartConfig {
    pub id: Option<i64>,
    pub project_id: String,
    pub table_path: String,
    pub chart_name: String,
    pub chart_type: String,
    pub config_json: String,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

pub fn save_chart_config(
    project_id: &str,
    table_path: &str,
    chart_name: &str,
    chart_type: &str,
    config_json: &str,
) -> Result<ChartConfig, CommandError> {
    let db_path = get_db_path()?;
    let conn = Connection::open(db_path)?;

    info!("[DB] Saving chart config '{}' for table '{}' in project '{}'", chart_name, table_path, project_id);

    // Insert or replace based on unique constraint (project_id, table_path, chart_name)
    conn.execute(
        "INSERT INTO table_charts (project_id, table_path, chart_name, chart_type, config_json)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(project_id, table_path, chart_name) DO UPDATE SET
         chart_type = excluded.chart_type,
         config_json = excluded.config_json,
         updated_at = CURRENT_TIMESTAMP",
        params![project_id, table_path, chart_name, chart_type, config_json],
    )?;

    // Fetch the saved item to return it
    let mut stmt = conn.prepare(
        "SELECT id, project_id, table_path, chart_name, chart_type, config_json, created_at, updated_at
         FROM table_charts
         WHERE project_id = ?1 AND table_path = ?2 AND chart_name = ?3"
    )?;

    let chart = stmt.query_row(params![project_id, table_path, chart_name], |row| {
        Ok(ChartConfig {
            id: row.get(0)?,
            project_id: row.get(1)?,
            table_path: row.get(2)?,
            chart_name: row.get(3)?,
            chart_type: row.get(4)?,
            config_json: row.get(5)?,
            created_at: row.get(6)?,
            updated_at: row.get(7)?,
        })
    })?;

    Ok(chart)
}

pub fn load_chart_configs(
    project_id: &str,
    table_path: &str,
) -> Result<Vec<ChartConfig>, CommandError> {
    let db_path = get_db_path()?;
    let conn = Connection::open(db_path)?;

    debug!("[DB] Loading chart configs for table '{}' in project '{}'", table_path, project_id);

    let mut stmt = conn.prepare(
        "SELECT id, project_id, table_path, chart_name, chart_type, config_json, created_at, updated_at
         FROM table_charts
         WHERE project_id = ?1 AND table_path = ?2
         ORDER BY updated_at DESC"
    )?;

    let chart_iter = stmt.query_map(params![project_id, table_path], |row| {
        Ok(ChartConfig {
            id: row.get(0)?,
            project_id: row.get(1)?,
            table_path: row.get(2)?,
            chart_name: row.get(3)?,
            chart_type: row.get(4)?,
            config_json: row.get(5)?,
            created_at: row.get(6)?,
            updated_at: row.get(7)?,
        })
    })?;

    let mut charts = Vec::new();
    for chart in chart_iter {
        charts.push(chart?);
    }

    Ok(charts)
}

pub fn delete_chart_config(
    project_id: &str,
    table_path: &str,
    chart_name: &str,
) -> Result<(), CommandError> {
    let db_path = get_db_path()?;
    let conn = Connection::open(db_path)?;

    info!("[DB] Deleting chart config '{}' for table '{}' in project '{}'", chart_name, table_path, project_id);

    conn.execute(
        "DELETE FROM table_charts WHERE project_id = ?1 AND table_path = ?2 AND chart_name = ?3",
        params![project_id, table_path, chart_name],
    )?;

    Ok(())
}

pub fn delete_all_charts_for_table(
    project_id: &str,
    table_path: &str,
) -> Result<(), CommandError> {
    let db_path = get_db_path()?;
    let conn = Connection::open(db_path)?;

    info!("[DB] Deleting all chart configs for table '{}' in project '{}'", table_path, project_id);

    conn.execute(
        "DELETE FROM table_charts WHERE project_id = ?1 AND table_path = ?2",
        params![project_id, table_path],
    )?;

    Ok(())
}
