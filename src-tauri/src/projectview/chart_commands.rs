// src-tauri/src/projectview/chart_commands.rs
use tauri::command;
use crate::projectview::chart_handler::{save_chart_config, load_chart_configs, delete_chart_config, ChartConfig};
use log::{info, error};

#[command]
pub async fn save_chart_config_command(
    project_id: String,
    table_path: String,
    chart_name: String,
    chart_type: String,
    config_json: String,
) -> Result<ChartConfig, String> {
    info!("[Command] Saving chart '{}'", chart_name);
    match save_chart_config(&project_id, &table_path, &chart_name, &chart_type, &config_json) {
        Ok(chart) => Ok(chart),
        Err(e) => {
            error!("[Command] Failed to save chart: {}", e);
            Err(e.to_string())
        }
    }
}

#[command]
pub async fn load_chart_configs_command(
    project_id: String,
    table_path: String,
) -> Result<Vec<ChartConfig>, String> {
    info!("[Command] Loading charts for table '{}'", table_path);
    match load_chart_configs(&project_id, &table_path) {
        Ok(charts) => Ok(charts),
        Err(e) => {
            error!("[Command] Failed to load charts: {}", e);
            Err(e.to_string())
        }
    }
}

#[command]
pub async fn delete_chart_config_command(
    project_id: String,
    table_path: String,
    chart_name: String,
) -> Result<(), String> {
    info!("[Command] Deleting chart '{}'", chart_name);
    match delete_chart_config(&project_id, &table_path, &chart_name) {
        Ok(_) => Ok(()),
        Err(e) => {
            error!("[Command] Failed to delete chart: {}", e);
            Err(e.to_string())
        }
    }
}
