// src-tauri/src/projectview/export_handler.rs

use crate::welcome::config::CommandError;
use crate::projectview::shared_types::{HARVEY_FILES_DIR, DOCS_DIR, TEMP_SUBDIR_DOCS};
use crate::projectview::shared_utils::ensure_base_asset_dirs;
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
};
use log::{info, warn, error, debug};
use tauri::AppHandle;
use tauri_plugin_shell::ShellExt;
use tauri_plugin_shell::process::CommandEvent;
use uuid::Uuid;
use html_escape::encode_text;

// --- Lexical Format Constants ---
const IS_BOLD: i64 = 1;
const IS_ITALIC: i64 = 1 << 1; // 2
const IS_STRIKETHROUGH: i64 = 1 << 2; // 4
const IS_UNDERLINE: i64 = 1 << 3; // 8
const IS_CODE: i64 = 1 << 4; // 16
const IS_SUBSCRIPT: i64 = 1 << 5; // 32
const IS_SUPERSCRIPT: i64 = 1 << 6; // 64
const IS_HIGHLIGHT: i64 = 1 << 7; // 128


/// Helper function to generate HTML from a parsed Lexical JSON value.
fn lexical_value_to_html(value: &Value) -> String {
    let mut html = String::new();

    if let Some(root) = value.get("root") {
        if let Some(children) = root.get("children").and_then(|c| c.as_array()) {
            for node in children {
                append_node_html(node, &mut html);
            }
        }
    } else {
        let plain_text = value.as_str().map(|s| s.to_string()).unwrap_or_else(|| value.to_string());
        html.push_str(&format!("<p>{}</p>", encode_text(&plain_text)));
    }

    html
}

/// Recursive helper to append HTML for a single Lexical node and its children.
fn append_node_html(node: &Value, html: &mut String) {
    if let Some(node_type) = node.get("type").and_then(|t| t.as_str()) {
        match node_type {
            "paragraph" => {
                 let style_attr = if let Some(format_align) = node.get("format").and_then(|f| f.as_str()) {
                     if !format_align.is_empty() {
                         format!(" style=\"text-align: {};\"", encode_text(format_align))
                     } else {
                         "".to_string()
                     }
                 } else {
                     "".to_string()
                 };

                html.push_str(&format!("<p{}>", style_attr));

                if let Some(children) = node.get("children").and_then(|c| c.as_array()) {
                    for child in children {
                        append_node_html(child, html);
                    }
                }
                html.push_str("</p>");
            }
            "heading" => {
                let tag = node.get("tag").and_then(|t| t.as_str()).unwrap_or("h1");
                html.push_str(&format!("<{}>", tag));
                 if let Some(children) = node.get("children").and_then(|c| c.as_array()) {
                    for child in children {
                        append_node_html(child, html);
                    }
                }
                html.push_str(&format!("</{}>", tag));
            }
             "list" => {
                let tag = node.get("tag").and_then(|t| t.as_str()).unwrap_or("ul");
                 html.push_str(&format!("<{}>", tag));
                 if let Some(children) = node.get("children").and_then(|c| c.as_array()) {
                    for child in children {
                        append_node_html(child, html);
                    }
                }
                 html.push_str(&format!("</{}>", tag));
             }
             "listitem" => {
                 html.push_str("<li>");
                 if let Some(children) = node.get("children").and_then(|c| c.as_array()) {
                    for child in children {
                        append_node_html(child, html);
                    }
                }
                 html.push_str("</li>");
             }
            "text" | "extended-text" => {
                if let Some(text_content) = node.get("text").and_then(|t| t.as_str()) {
                    let format_flags = node.get("format").and_then(|f| f.as_i64()).unwrap_or(0);
                    let mut format_tags_to_close = Vec::new();
                    let mut format_tags_to_open = String::new();
                    // Build opening tags for standard formats (excluding highlight)
                    if format_flags & IS_BOLD != 0 { format_tags_to_open.push_str("<b>"); format_tags_to_close.push("</b>"); }
                    if format_flags & IS_ITALIC != 0 { format_tags_to_open.push_str("<i>"); format_tags_to_close.push("</i>"); }
                    if format_flags & IS_UNDERLINE != 0 { format_tags_to_open.push_str("<u>"); format_tags_to_close.push("</u>"); }
                    if format_flags & IS_STRIKETHROUGH != 0 { format_tags_to_open.push_str("<s>"); format_tags_to_close.push("</s>"); }
                    if format_flags & IS_CODE != 0 { format_tags_to_open.push_str("<code>"); format_tags_to_close.push("</code>"); }
                    if format_flags & IS_SUBSCRIPT != 0 { format_tags_to_open.push_str("<sub>"); format_tags_to_close.push("</sub>"); }
                    if format_flags & IS_SUPERSCRIPT != 0 { format_tags_to_open.push_str("<sup>"); format_tags_to_close.push("</sup>"); }

                    let style_str = node.get("style").and_then(|s| s.as_str()).unwrap_or("");

                    // Parse CSS style for color and background-color
                    let mut text_color: Option<&str> = None;
                    let mut has_highlight_flag = format_flags & IS_HIGHLIGHT != 0;
                    for decl in style_str.split(';').map(str::trim) {
                        if let Some(value) = decl.strip_prefix("color:") {
                            text_color = Some(value.trim());
                        } else if let Some(_) = decl.strip_prefix("background-color:") {
                            has_highlight_flag = true;
                        }
                    }

                    let escaped_text = encode_text(text_content);

                    // Open highlight tag if needed
                    if has_highlight_flag {
                        html.push_str("<mark>");
                    }
                    // Open font color tag if needed
                    if let Some(color) = text_color {
                        html.push_str(&format!("<font color=\"{}\">", encode_text(color)));
                    }
                    // Open format tags
                    html.push_str(&format_tags_to_open);
                    // Insert the actual text
                    html.push_str(&escaped_text);
                    // Close format tags
                    while let Some(tag) = format_tags_to_close.pop() {
                        html.push_str(tag);
                    }
                    // Close font tag if used
                    if text_color.is_some() {
                        html.push_str("</font>");
                    }
                    // Close highlight tag if used
                    if has_highlight_flag {
                        html.push_str("</mark>");
                    }
                }
            }
            "linebreak" => {
                html.push_str("<br />");
            }
            "link" => {
                 let url = node.get("url").and_then(|u| u.as_str()).unwrap_or("#");
                 html.push_str(&format!("<a href=\"{}\">", encode_text(url)));
                 if let Some(children) = node.get("children").and_then(|c| c.as_array()) {
                    for child in children {
                        append_node_html(child, html);
                    }
                 }
                 html.push_str("</a>");
            }
            _ => {
                warn!("Unknown lexical node type encountered in HTML export: {}", node_type);
                if let Some(children) = node.get("children").and_then(|c| c.as_array()) {
                    for child in children {
                        append_node_html(child, html);
                    }
                } else if let Some(text_content) = node.get("text").and_then(|t| t.as_str()) {
                     html.push_str(&encode_text(text_content).to_string());
                }
            }
        }
    } else {
         warn!("Lexical node missing 'type' field: {:?}", node);
    }
}

/// Attempts to parse a string as Lexical JSON and convert it to HTML.
fn convert_lexical_or_plain_text_to_html(text_content: &str) -> String {
    match serde_json::from_str::<Value>(text_content) {
        Ok(parsed_json) => {
            if parsed_json.get("root").and_then(|r| r.get("children")).is_some() {
                lexical_value_to_html(&parsed_json)
            } else {
                format!("<p>{}</p>", encode_text(text_content))
            }
        }
        Err(_) => {
             if text_content.trim().starts_with('<') && text_content.trim().ends_with('>') {
                 text_content.to_string()
             } else {
                  format!("<p>{}</p>", encode_text(text_content))
             }
        }
    }
}


/// Generates a unique temp path for a given extension in Documents/.tmp/
fn get_unique_temp_path_for_conversion(
    base_dir: &Path,
    prefix: &str,
    extension: &str,
) -> Result<PathBuf, CommandError> {
    let temp_dir = base_dir.join(HARVEY_FILES_DIR).join(DOCS_DIR).join(TEMP_SUBDIR_DOCS);
    fs::create_dir_all(&temp_dir).map_err(|e| CommandError::from(format!("Failed to create temp dir {}: {}", temp_dir.display(), e)))?;

    let uid = Uuid::new_v4();
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);

    let safe_prefix = prefix
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
        .collect::<String>();

    let filename = format!("{}_{}_{}.{}", safe_prefix, ts, uid, extension);
    let full_path = temp_dir.join(&filename);
    debug!("Generated temp conversion path: {}", full_path.display());
    Ok(full_path)
}


#[tauri::command]
pub async fn export_transcript_to_docx(
    app_handle: AppHandle,
    transcript_json_path_str: String,
    output_path_str: String,
    layout_choice: Option<String>, // Added layout_choice parameter
) -> Result<String, CommandError> {
    let current_layout = layout_choice.unwrap_or_else(|| "Layout2".to_string());
    info!(
        "[export_transcript_to_docx] Starting export from JSON: {}, Target DOCX: {}, Layout: {}",
        transcript_json_path_str, output_path_str, current_layout
    );

    let source_path = PathBuf::from(&transcript_json_path_str);
    if !source_path.exists() || !source_path.is_file() {
        let msg = format!("Transcript JSON file not found: {}", transcript_json_path_str);
        error!("[export_transcript_to_docx] {}", msg);
        return Err(CommandError::from(msg));
    }

     let base_dir = source_path
         .parent()
         .and_then(|p| p.parent())
         .and_then(|p| p.parent())
         .and_then(|p| p.parent())
         .and_then(|p| p.parent())
         .ok_or_else(|| {
             let msg = format!("Could not determine project base directory from transcript path: {}", transcript_json_path_str);
             error!("[export_transcript_to_docx] {}", msg);
             CommandError::from(msg)
         })?;

    info!("[export_transcript_to_docx] Determined project base directory: {}", base_dir.display());

    if let Err(e) = ensure_base_asset_dirs(&base_dir) {
         error!("[export_transcript_to_docx] Failed to ensure base asset dirs: {:?}", e);
    }


    let output_path = PathBuf::from(&output_path_str);
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).map_err(|e| CommandError::from(format!("Failed to create output directory {}: {}", parent.display(), e)))?;
    } else {
        let msg = format!("Invalid output path (no parent directory): {}", output_path_str);
        error!("[export_transcript_to_docx] {}", msg);
        return Err(CommandError::from(msg));
    }

    let json_content = fs::read_to_string(&source_path)?;
    let json_value: Value = serde_json::from_str(&json_content)
        .map_err(|e| CommandError::from(format!("Failed to parse transcript JSON: {}", e)))?;

    let entries: Vec<Value> = if let Value::Array(arr) = json_value {
        arr
    } else if let Some(root) = json_value.get("root") {
        // Lexical JSON table format
        let children = root.get("children")
            .and_then(|c| c.as_array())
            .ok_or_else(|| CommandError::from("Invalid Lexical JSON: missing root.children"))?;
        let table_node = children.get(0)
            .ok_or_else(|| CommandError::from("Invalid Lexical JSON: missing table node"))?;
        if table_node.get("type").and_then(|t| t.as_str()) != Some("table") {
            return Err(CommandError::from("Invalid Lexical JSON: first child is not a table"));
        }
        let rows = table_node.get("children")
            .and_then(|r| r.as_array())
            .ok_or_else(|| CommandError::from("Invalid Lexical JSON: missing table children"))?;
        // Helper to parse timestamp range "mm:ss.mmm - mm:ss.mmm"
        fn parse_ts_range(range: &str) -> (f64, f64) {
            let parts: Vec<&str> = range.split(" - ").collect();
            fn parse_one(s: &str) -> f64 {
                let parts: Vec<&str> = s.split(':').collect();
                match parts.len() {
                    2 => {
                        let m: f64 = parts[0].parse().unwrap_or(0.0);
                        let s: f64 = parts[1].parse().unwrap_or(0.0);
                        m * 60.0 + s
                    }
                    3 => {
                        let h: f64 = parts[0].parse().unwrap_or(0.0);
                        let m: f64 = parts[1].parse().unwrap_or(0.0);
                        let s: f64 = parts[2].parse().unwrap_or(0.0);
                        h * 3600.0 + m * 60.0 + s
                    }
                    _ => 0.0,
                }
            }
            if parts.len() == 2 {
                (parse_one(parts[0]), parse_one(parts[1]))
            } else {
                (0.0, 0.0)
            }
        }
        let mut segs = Vec::new();
        for row in rows.iter().skip(1) {
            if let Some(cells) = row.get("children").and_then(|c| c.as_array()) {
                // Extract cell texts
                let texts: Vec<String> = cells.iter().map(|cell| {
                    cell.get("children").and_then(|p| p.as_array())
                        .and_then(|ps| ps.get(0))
                        .and_then(|p| p.get("children"))
                        .and_then(|ts| ts.as_array())
                        .and_then(|ts| ts.get(0))
                        .and_then(|t| t.get("text"))
                        .and_then(|t| t.as_str())
                        .unwrap_or("")
                        .to_string()
                }).collect();
                let (start, end) = parse_ts_range(&texts[1]);
                let seg = json!({
                    "start_time": start,
                    "end_time": end,
                    "speaker": texts.get(2).cloned().unwrap_or_default(),
                    "text": texts.get(3).cloned().unwrap_or_default(),
                });
                segs.push(seg);
            }
        }
        segs
    } else {
        return Err(CommandError::from("Transcript JSON must be an array or Lexical JSON table"));
    };

    let mut html_output = String::new();
    html_output.push_str("<!DOCTYPE html>\n");
    html_output.push_str("<html><head><meta charset=\"utf-8\"/><style>\n");
    html_output.push_str("body { \
        font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, \
        'Liberation Mono', 'Courier New', monospace; \
        font-size: 14px; \
        line-height: 1.5; \
    }\n");
    html_output.push_str("</style></head><body>\n");

    html_output.push_str("<table style=\"table-layout:fixed; width:100%; border-collapse: collapse;\">\n");

    // Determine colgroup based on layout_choice
    match current_layout.as_str() {
        "Layout1" => { // | No | Timestamp | Speaker | Text | (5%, 15%, 15%, 65%)
            html_output.push_str("<colgroup>\n");
            html_output.push_str("  <col style=\"width:5%\" />\n");
            html_output.push_str("  <col style=\"width:15%\" />\n");
            html_output.push_str("  <col style=\"width:15%\" />\n");
            html_output.push_str("  <col style=\"width:65%\" />\n");
            html_output.push_str("</colgroup>\n");
        }
        "Layout2" => { // | No | Timestamp | then | Speaker | Text | (20%, 80%) - Default
            html_output.push_str("<colgroup>\n");
            html_output.push_str("  <col style=\"width:20%\" />\n");
            html_output.push_str("  <col style=\"width:80%\" />\n");
            html_output.push_str("</colgroup>\n");
        }
        "Layout3" => { // | Timestamp Speaker | then | Text | (100%)
            html_output.push_str("<colgroup>\n");
            html_output.push_str("  <col style=\"width:100%\" />\n");
            html_output.push_str("</colgroup>\n");
        }
        "Layout4" => { // | Speaker | Text | (25%, 75%)
            html_output.push_str("<colgroup>\n");
            html_output.push_str("  <col style=\"width:25%\" />\n");
            html_output.push_str("  <col style=\"width:75%\" />\n");
            html_output.push_str("</colgroup>\n");
        }
        "Layout5" => { // | Text | (100%)
            html_output.push_str("<colgroup>\n");
            html_output.push_str("  <col style=\"width:100%\" />\n");
            html_output.push_str("</colgroup>\n");
        }
        _ => { // Default to Layout2 if unknown
            html_output.push_str("<colgroup>\n");
            html_output.push_str("  <col style=\"width:20%\" />\n");
            html_output.push_str("  <col style=\"width:80%\" />\n");
            html_output.push_str("</colgroup>\n");
        }
    }
    html_output.push_str("  <tbody>\n");

    // Helper to format timestamps as mm:ss.mmm or hh:mm:ss.mmm
    let format_ts = |seconds: f64| -> String {
        if seconds.is_nan() || seconds < 0.0 {
            return "00:00.000".to_string();
        }
        let total_ms = (seconds * 1000.0).round() as u64;
        let ms = total_ms % 1000;
        let total_s = total_ms / 1000;
        if total_s < 3600 { // Less than an hour
            let minutes = total_s / 60;
            let secs = total_s % 60;
            format!("{:02}:{:02}.{:03}", minutes, secs, ms)
        } else { // Hour included
            let hours = total_s / 3600;
            let minutes = (total_s % 3600) / 60;
            let secs = total_s % 60;
            format!("{:02}:{:02}:{:02}.{:03}", hours, minutes, secs, ms)
        }
    };

    for (index, entry) in entries.iter().enumerate() {
        let segment_number = index + 1;
        let start = entry.get("start_time").and_then(Value::as_f64).unwrap_or(0.0);
        let end = entry.get("end_time").and_then(Value::as_f64).unwrap_or(0.0);
        let timestamp_str = format!("{} - {}", format_ts(start), format_ts(end));
        let raw_speaker = entry.get("speaker").and_then(Value::as_str).unwrap_or("Unknown");
        let speaker_display_with_colon = if raw_speaker.chars().count() > 12 {
            format!("{}:", raw_speaker.chars().take(12).collect::<String>() + "...")
        } else {
            format!("{}:", raw_speaker)
        };
        let speaker_display_no_colon = if raw_speaker.chars().count() > 12 {
            format!("{}", raw_speaker.chars().take(12).collect::<String>() + "...")
        } else {
            raw_speaker.to_string()
        };
        let raw_text_content = entry.get("text").and_then(Value::as_str).unwrap_or("");
        let segment_html_content = convert_lexical_or_plain_text_to_html(raw_text_content);

        match current_layout.as_str() {
            "Layout1" => { // | No | Timestamp | Speaker | Text |
                html_output.push_str("    <tr style=\"page-break-inside: avoid;\">\n");
                html_output.push_str(&format!("      <td style=\"vertical-align:top; padding: 4px; border: 1px solid #eee; font-size: 0.9em; color: #555;\">{}</td>\n", segment_number));
                html_output.push_str(&format!("      <td style=\"vertical-align:top; padding: 4px; border: 1px solid #eee; font-size: 0.9em; color: #555;\">{}</td>\n", encode_text(&timestamp_str)));
                html_output.push_str(&format!("      <td style=\"vertical-align:top; padding: 4px; border: 1px solid #eee; font-weight: bold;\">{}</td>\n", encode_text(&speaker_display_no_colon)));
                html_output.push_str(&format!("      <td style=\"vertical-align:top; padding: 4px; border: 1px solid #eee;\">{}</td>\n", segment_html_content));
                html_output.push_str("    </tr>\n");
            }
            "Layout2" => { // | No | Timestamp | then | Speaker | Text | (Default)
                html_output.push_str("    <tr style=\"page-break-inside: avoid;\">\n");
                html_output.push_str(&format!("      <td style=\"vertical-align:top; padding: 4px; border: 1px solid #eee; font-size: 0.9em; color: #555;\">{}</td>\n", segment_number));
                html_output.push_str(&format!("      <td style=\"vertical-align:top; padding: 4px; border: 1px solid #eee; font-size: 0.9em; color: #555;\">{}</td>\n", encode_text(&timestamp_str)));
                html_output.push_str("    </tr>\n");
                html_output.push_str("    <tr style=\"page-break-inside: avoid;\">\n");
                html_output.push_str(&format!("      <td style=\"vertical-align:top; padding: 4px; border: 1px solid #eee; font-weight: bold;\">{}</td>\n", encode_text(&speaker_display_with_colon)));
                html_output.push_str(&format!("      <td style=\"vertical-align:top; padding: 4px; border: 1px solid #eee;\">{}</td>\n", segment_html_content));
                html_output.push_str("    </tr>\n");
            }
            "Layout3" => { // | Timestamp Speaker | then | Text |
                html_output.push_str("    <tr style=\"page-break-inside: avoid;\">\n");
                html_output.push_str(&format!("      <td style=\"vertical-align:top; padding: 4px; border: 1px solid #eee; font-size: 0.9em; color: #555;\">{} {}</td>\n", encode_text(&timestamp_str), encode_text(&speaker_display_no_colon)));
                html_output.push_str("    </tr>\n");
                html_output.push_str("    <tr style=\"page-break-inside: avoid;\">\n");
                html_output.push_str(&format!("      <td style=\"vertical-align:top; padding: 4px; border: 1px solid #eee;\">{}</td>\n", segment_html_content));
                html_output.push_str("    </tr>\n");
            }
            "Layout4" => { // | Speaker | Text |
                html_output.push_str("    <tr style=\"page-break-inside: avoid;\">\n");
                html_output.push_str(&format!("      <td style=\"vertical-align:top; padding: 4px; border: 1px solid #eee; font-weight: bold;\">{}</td>\n", encode_text(&speaker_display_no_colon)));
                html_output.push_str(&format!("      <td style=\"vertical-align:top; padding: 4px; border: 1px solid #eee;\">{}</td>\n", segment_html_content));
                html_output.push_str("    </tr>\n");
            }
            "Layout5" => { // | Text |
                html_output.push_str("    <tr style=\"page-break-inside: avoid;\">\n");
                html_output.push_str(&format!("      <td style=\"vertical-align:top; padding: 4px; border: 1px solid #eee;\">{}</td>\n", segment_html_content));
                html_output.push_str("    </tr>\n");
            }
            _ => { // Fallback to Layout2 if layout_choice is unknown
                html_output.push_str("    <tr style=\"page-break-inside: avoid;\">\n");
                html_output.push_str(&format!("      <td style=\"vertical-align:top; padding: 4px; border: 1px solid #eee; font-size: 0.9em; color: #555;\">{}</td>\n", segment_number));
                html_output.push_str(&format!("      <td style=\"vertical-align:top; padding: 4px; border: 1px solid #eee; font-size: 0.9em; color: #555;\">{}</td>\n", encode_text(&timestamp_str)));
                html_output.push_str("    </tr>\n");
                html_output.push_str("    <tr style=\"page-break-inside: avoid;\">\n");
                html_output.push_str(&format!("      <td style=\"vertical-align:top; padding: 4px; border: 1px solid #eee; font-weight: bold;\">{}</td>\n", encode_text(&speaker_display_with_colon)));
                html_output.push_str(&format!("      <td style=\"vertical-align:top; padding: 4px; border: 1px solid #eee;\">{}</td>\n", segment_html_content));
                html_output.push_str("    </tr>\n");
            }
        }
    }

    html_output.push_str("  </tbody>\n");
    html_output.push_str("</table>\n");
    html_output.push_str("</body></html>\n");

    let stem = source_path.file_stem().and_then(|s| s.to_str()).unwrap_or("transcript_export");
    let temp_html_path = get_unique_temp_path_for_conversion(&base_dir, stem, "html")?;
    debug!("[export_transcript_to_docx] Writing generated HTML table to temp file: {}", temp_html_path.display());
    fs::write(&temp_html_path, &html_output)?;

    let temp_docx_path = get_unique_temp_path_for_conversion(&base_dir, stem, "docx")?;
    // Path to custom reference DOCX with desired styles (compile-time asset path)
    let reference_docx = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .join("reference.docx")
        .to_string_lossy()
        .to_string();
    let pandoc_args = vec![
        temp_html_path.to_string_lossy().to_string(),
        "-f".to_string(),
        "html".to_string(),
        "-t".to_string(),
        "docx".to_string(),
        "--reference-doc".to_string(),
        reference_docx.clone(),
        "-o".to_string(),
        temp_docx_path.to_string_lossy().to_string(),
    ];

    info!("[export_transcript_to_docx] Executing Pandoc: pandoc {}", pandoc_args.join(" "));

    let (mut rx, _child) = app_handle
        .shell()
        .sidecar("pandoc")?
        .args(&pandoc_args)
        .spawn()
        .map_err(|e| {
            let msg = format!("Pandoc execution failed: {}. Is Pandoc configured as a sidecar?", e);
            error!("[export_transcript_to_docx] Pandoc sidecar spawn failed: {}", e);
            CommandError::from(msg)
        })?;

    let mut pandoc_stderr = String::new();
    let mut exit_code = None;

    while let Some(event) = rx.recv().await {
        match event {
            CommandEvent::Stdout(line) => { debug!("[Pandoc STDOUT] {}", String::from_utf8_lossy(&line)); },
            CommandEvent::Stderr(line) => {
                let line_str = String::from_utf8_lossy(&line);
                debug!("[Pandoc STDERR] {}", line_str);
                pandoc_stderr.push_str(&line_str);
                pandoc_stderr.push('\n');
            },
            CommandEvent::Error(e) => {
                error!("[export_transcript_to_docx] Pandoc command event error: {}", e);
                pandoc_stderr.push_str(&format!("Command event error: {}\n", e));
            },
            CommandEvent::Terminated(payload) => {
                 debug!("[export_transcript_to_docx] Pandoc terminated with payload: {:?}", payload);
                exit_code = payload.code;
                 if payload.signal.is_some() {
                     let signal_msg = format!("Pandoc terminated by signal: {:?}", payload.signal);
                     error!("[export_transcript_to_docx] {}", signal_msg);
                     pandoc_stderr.push_str(&signal_msg);
                 }
                break;
            }
            _ => { }
        }
    }

    if exit_code != Some(0) {
        let err_msg = format!(
            "Pandoc conversion failed (exit code {:?}). Stderr:\n{}",
            exit_code,
            pandoc_stderr
        );
        error!("[export_transcript_to_docx] {}", err_msg);
        let _ = fs::remove_file(&temp_html_path);
        let _ = fs::remove_file(&temp_docx_path);
        return Err(CommandError::from(err_msg));
    }

    info!("[export_transcript_to_docx] Pandoc conversion successful.");

    debug!("[export_transcript_to_docx] Copying temp DOCX {} to final path {}", temp_docx_path.display(), output_path.display());
    fs::copy(&temp_docx_path, &output_path).map_err(|e| {
        let msg = format!(
            "Failed to copy temporary DOCX {} to final output path {}: {}",
            temp_docx_path.display(),
            output_path.display(),
            e
        );
        error!("[export_transcript_to_docx] {}", msg);
         let _ = fs::remove_file(&temp_html_path);
         let _ = fs::remove_file(&temp_docx_path);
        CommandError::from(msg)
    })?;

    info!("[export_transcript_to_docx] DOCX file successfully copied to {}", output_path.display());

    debug!("[export_transcript_to_docx] Cleaning up temporary files...");
    if let Err(e) = fs::remove_file(&temp_html_path) {
        warn!("[export_transcript_to_docx] Failed to delete temporary HTML file {}: {}", temp_html_path.display(), e);
    } else {
        debug!("[export_transcript_to_docx] Deleted temporary HTML file: {}", temp_html_path.display());
    }
    if let Err(e) = fs::remove_file(&temp_docx_path) {
        warn!("[export_transcript_to_docx] Failed to delete temporary DOCX file {}: {}", temp_docx_path.display(), e);
    } else {
        debug!("[export_transcript_to_docx] Deleted temporary DOCX file: {}", temp_docx_path.display());
    }

    info!(
        "[export_transcript_to_docx] Export process finished successfully. DOCX saved to {}",
        output_path.display()
    );
    Ok(output_path.to_string_lossy().to_string())
}

#[derive(serde::Deserialize, Debug)]
struct Segment {
    start_time: f64,
    end_time: f64,
    speaker: Option<String>,
    text: String,
}

// Helper function to format seconds to HH:MM:SS,mmm
fn format_srt_timestamp(seconds: f64) -> String {
    if seconds.is_nan() || seconds < 0.0 {
        return "00:00:00,000".to_string();
    }
    let total_ms = (seconds * 1000.0).round() as u64;
    let ms = total_ms % 1000;
    let total_s = total_ms / 1000;
    let s = total_s % 60;
    let total_m = total_s / 60;
    let m = total_m % 60;
    let h = total_m / 60;
    format!("{:02}:{:02}:{:02},{:03}", h, m, s, ms)
}

/// Extracts plain text from a Lexical JSON structure.
fn extract_plain_text_from_lexical_value(value: &Value, text_buffer: &mut String) {
    if let Some(node_type) = value.get("type").and_then(|t| t.as_str()) {
        match node_type {
            "text" | "extended-text" => {
                if let Some(text_content) = value.get("text").and_then(|t| t.as_str()) {
                    text_buffer.push_str(text_content);
                }
            }
            "linebreak" => {
                text_buffer.push_str("\n");
            }
            "paragraph" | "heading" | "list" | "listitem" | "quote" | "link" | "table" | "tablecell" | "tablerow" => {
                if let Some(children) = value.get("children").and_then(|c| c.as_array()) {
                    for (i, child) in children.iter().enumerate() {
                        extract_plain_text_from_lexical_value(child, text_buffer);
                        if node_type == "paragraph" && i < children.len() -1 { // Add space between children of a paragraph unless it's the last one.
                           // This might need refinement based on desired paragraph spacing in SRT.
                           // For SRT, often multiple "paragraphs" in Lexical might just be one continuous text block.
                        }
                    }
                }
                 // Add a space after block elements like paragraphs if they are not followed by another block or to ensure separation.
                if node_type == "paragraph" && !text_buffer.ends_with("\n") && !text_buffer.is_empty() {
                    // text_buffer.push_str(" "); // Or "\n" if paragraphs should be new lines in SRT
                }
            }
            _ => { // For unknown types, try to process children if any
                if let Some(children) = value.get("children").and_then(|c| c.as_array()) {
                    for child in children {
                        extract_plain_text_from_lexical_value(child, text_buffer);
                    }
                }
            }
        }
    } else if let Some(children) = value.get("root").and_then(|r| r.get("children")).and_then(|c| c.as_array()) {
        // This handles the case where the entire editor state is passed
        for (i, child) in children.iter().enumerate() {
            extract_plain_text_from_lexical_value(child, text_buffer);
            // Add a newline between top-level block nodes (e.g., paragraphs)
            if i < children.len() - 1 {
                 if let Some(child_node_type) = child.get("type").and_then(|t| t.as_str()) {
                    if child_node_type == "paragraph" && !text_buffer.ends_with('\n') {
                         text_buffer.push_str("\n");
                    }
                 }
            }
        }
    } else if value.is_string() && value.as_str().map_or(false, |s| s.trim().is_empty() || (!s.contains("{") && !s.contains("}")) ) {
        // If it's a plain string (likely already plain text or empty)
        text_buffer.push_str(value.as_str().unwrap_or(""));
    }
    // If it's some other JSON structure not matching Lexical, it will be ignored.
}


fn get_plain_text_for_srt(text_content: &str) -> String {
    match serde_json::from_str::<Value>(text_content) {
        Ok(parsed_json) => {
            // Check if it's a Lexical root structure
            if parsed_json.get("root").and_then(|r| r.get("children")).is_some() {
                let mut buffer = String::new();
                extract_plain_text_from_lexical_value(&parsed_json, &mut buffer);
                return buffer.trim().to_string();
            }
            // If it's some other JSON, or not a Lexical root, try to treat as plain text
            // or return an indication of non-Lexical JSON. For SRT, we prefer plain.
            if parsed_json.is_string() {
                return parsed_json.as_str().unwrap_or("").trim().to_string();
            }
            // Fallback for non-string JSON or non-Lexical root: return original string, trimmed.
            // This might happen if the content was already plain text but got wrapped in quotes by mistake.
            text_content.trim().to_string()
        }
        Err(_) => {
            // Not valid JSON, assume it's already plain text
            text_content.trim().to_string()
        }
    }
}

// The duplicate function definition that was here (around line 725) is removed.
// The original definition at line 628 is kept.

#[tauri::command]
pub async fn export_transcript_to_srt(
    _app_handle: AppHandle, // Not directly used but good practice for tauri commands
    output_path_str: String,
    segments_json_str: String,
) -> Result<String, CommandError> {
    info!("[export_transcript_to_srt] Exporting to SRT: {}", output_path_str);

    let segments: Vec<Segment> = serde_json::from_str(&segments_json_str)
        .map_err(|e| CommandError::from(format!("Failed to parse segments JSON for SRT: {}", e)))?;

    if segments.is_empty() {
        return Err(CommandError::from("No segments provided for SRT export."));
    }

    let mut srt_content = String::new();
    for (index, segment) in segments.iter().enumerate() {
        srt_content.push_str(&(index + 1).to_string());
        srt_content.push_str("\n");

        let start_ts = format_srt_timestamp(segment.start_time);
        let end_ts = format_srt_timestamp(segment.end_time);
        srt_content.push_str(&format!("{} --> {}\n", start_ts, end_ts));

        let plain_text = get_plain_text_for_srt(&segment.text);

        // Prepend speaker to text if speaker exists and is not empty
        let text_line = if let Some(speaker_name) = &segment.speaker {
            if !speaker_name.trim().is_empty() {
                format!("{}: {}", speaker_name.trim(), plain_text)
            } else {
                plain_text
            }
        } else {
            plain_text
        };
        srt_content.push_str(&text_line);
        srt_content.push_str("\n\n"); // Two newlines to separate blocks
    }

    fs::write(&output_path_str, srt_content)
        .map_err(|e| CommandError::from(format!("Failed to write SRT file {}: {}", output_path_str, e)))?;

    info!("[export_transcript_to_srt] SRT export successful to {}", output_path_str);
    Ok(output_path_str)
}