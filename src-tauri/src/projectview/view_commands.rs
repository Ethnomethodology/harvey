use crate::welcome::config::CommandError;
use crate::projectview::view_handler::{self, ViewConfig};
use log::info;

#[tauri::command]
pub async fn save_table_view_command(
    project_id: String,
    table_path: String,
    view_name: String,
    view_type: String,
    config_json: String,
) -> Result<ViewConfig, CommandError> {
    info!("Saving table view: {} for table: {} in project: {}", view_name, table_path, project_id);
    view_handler::save_table_view(&project_id, &table_path, &view_name, &view_type, &config_json)
}

#[tauri::command]
pub async fn load_table_views_command(
    project_id: String,
    table_path: String,
) -> Result<Vec<ViewConfig>, CommandError> {
    info!("Loading table views for table: {} in project: {}", table_path, project_id);
    view_handler::load_table_views(&project_id, &table_path)
}

#[tauri::command]
pub async fn delete_table_view_command(
    project_id: String,
    table_path: String,
    view_name: String,
    project_xml_path_str: String,
) -> Result<(), CommandError> {
    info!("Deleting table view: {} for table: {} in project: {}", view_name, table_path, project_id);
    view_handler::delete_table_view(&project_id, &table_path, &view_name, &project_xml_path_str)
}