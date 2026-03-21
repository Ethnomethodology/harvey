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
use tauri::{AppHandle, Runtime};
use tauri::Manager;
use tauri_plugin_shell::process::CommandEvent;
use crate::welcome::python_env::get_python_command;
use uuid::Uuid;
use html_escape::encode_text;
use regex::Regex;

// --- Lexical Format Constants ---
const IS_BOLD: i64 = 1;
const IS_ITALIC: i64 = 1 << 1; // 2
const IS_STRIKETHROUGH: i64 = 1 << 2; // 4
const IS_UNDERLINE: i64 = 1 << 3; // 8
const IS_CODE: i64 = 1 << 4; // 16
const IS_SUBSCRIPT: i64 = 1 << 5; // 32
const IS_SUPERSCRIPT: i64 = 1 << 6; // 64
const IS_HIGHLIGHT: i64 = 1 << 7; // 128


fn ensure_hex_color(color: &str) -> Option<String> {
    let trimmed = color.trim();
    if trimmed.is_empty() || trimmed == "transparent" {
        return None;
    }
    if trimmed.starts_with('#') {
        let hex = trimmed.trim_start_matches('#');
        if hex.len() == 6 {
            return Some(trimmed.to_uppercase());
        }
        if hex.len() == 3 {
             let r = &hex[0..1];
             let g = &hex[1..2];
             let b = &hex[2..3];
             return Some(format!("#{}{}{}{}{}{}", r, r, g, g, b, b).to_uppercase());
        }
    }
    if let Ok(re) = Regex::new(r"rgb\s*\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)\s*\)") {
        if let Some(caps) = re.captures(trimmed) {
             let r = caps[1].parse::<u8>().unwrap_or(0);
             let g = caps[2].parse::<u8>().unwrap_or(0);
             let b = caps[3].parse::<u8>().unwrap_or(0);
             return Some(format!("#{:02X}{:02X}{:02X}", r, g, b));
        }
    }
    match trimmed.to_lowercase().as_str() {
        "black" => Some("#000000".to_string()),
        "white" => Some("#FFFFFF".to_string()),
        "red" => Some("#FF0000".to_string()),
        "green" => Some("#00FF00".to_string()),
        "blue" => Some("#0000FF".to_string()),
        "yellow" => Some("#FFFF00".to_string()),
        "magenta" | "fuchsia" => Some("#FF00FF".to_string()),
        "cyan" | "aqua" => Some("#00FFFF".to_string()),
        "gray" | "grey" => Some("#808080".to_string()),
        "orange" => Some("#FFA500".to_string()),
        "purple" => Some("#800080".to_string()),
        _ => None,
    }
}

fn sanitize_font_family(font: &str) -> Option<String> {
    let trimmed = font.trim();
    if trimmed.is_empty() { return None; }
    let first = trimmed.split(',').next().unwrap_or(trimmed).trim();
    let sanitized = first.replace("\"", "").replace("'", "");
    if sanitized.is_empty() { return None; }
    Some(sanitized)
}

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

                    // Parse CSS style for color, background-color, font-family, and font-size
                    let mut text_color: Option<String> = None;
                    let mut font_family: Option<String> = None;
                    let mut font_size: Option<String> = None;
                    let mut highlight_color: Option<String> = None;

                    let mut has_highlight_flag = format_flags & IS_HIGHLIGHT != 0;

                    for decl in style_str.split(';').map(str::trim) {
                        if let Some(value) = decl.strip_prefix("color:") {
                            text_color = ensure_hex_color(value.trim());
                        } else if let Some(value) = decl.strip_prefix("font-family:") {
                            font_family = sanitize_font_family(value.trim());
                        } else if let Some(value) = decl.strip_prefix("font-size:") {
                            font_size = Some(value.trim().to_string());
                        } else if let Some(value) = decl.strip_prefix("background-color:") {
                            let bg = value.trim();
                            if !bg.is_empty() && bg != "transparent" {
                                has_highlight_flag = true;
                                highlight_color = ensure_hex_color(bg);
                            }
                        }
                    }

                    // Prepare data attributes for the Lua filter
                    let mut span_attrs = String::new();
                    if let Some(c) = text_color {
                        span_attrs.push_str(&format!(" data-color=\"{}\"", c));
                    }
                    if let Some(f) = font_family {
                        span_attrs.push_str(&format!(" data-font-family=\"{}\"", encode_text(&f)));
                    }
                    if let Some(s) = font_size {
                        span_attrs.push_str(&format!(" data-font-size=\"{}\"", encode_text(&s)));
                    }
                    if has_highlight_flag {
                         let hc = highlight_color.unwrap_or_else(|| "#FFFF00".to_string());
                         span_attrs.push_str(&format!(" data-highlight=\"{}\"", hc));
                    }

                    // Open wrapper span if attributes exist
                    if !span_attrs.is_empty() {
                        html.push_str(&format!("<span{}>", span_attrs));
                    }

                    // Open format tags (B, I, U, S, etc.)
                    html.push_str(&format_tags_to_open);

                    // Insert the actual text
                    html.push_str(&encode_text(text_content));

                    // Close format tags
                    while let Some(tag) = format_tags_to_close.pop() {
                        html.push_str(tag);
                    }

                    // Close wrapper span
                    if !span_attrs.is_empty() {
                        html.push_str("</span>");
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
            "table" => {
                 html.push_str("<table border=\"1\"><tbody>");
                 if let Some(children) = node.get("children").and_then(|c| c.as_array()) {
                    for child in children {
                        append_node_html(child, html);
                    }
                 }
                 html.push_str("</tbody></table>");
            }
            "tablerow" => {
                 html.push_str("<tr>");
                 if let Some(children) = node.get("children").and_then(|c| c.as_array()) {
                    for child in children {
                        append_node_html(child, html);
                    }
                 }
                 html.push_str("</tr>");
            }
            "tablecell" => {
                 html.push_str("<td>");
                 if let Some(children) = node.get("children").and_then(|c| c.as_array()) {
                    for child in children {
                        append_node_html(child, html);
                    }
                 }
                 html.push_str("</td>");
            }
            "image" => {
                 let filename = node.get("filename").and_then(|f| f.as_str()).unwrap_or("image.png");
                 let alt = node.get("altText").and_then(|a| a.as_str()).unwrap_or("Image");
                 html.push_str(&format!("<img src=\"attachments/{}\" alt=\"{}\" />", encode_text(filename), encode_text(alt)));
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
pub async fn export_transcript_to_docx<R: Runtime>(
    app_handle: AppHandle<R>,
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
        let table_node = children.iter().find(|node| node.get("type").and_then(|t| t.as_str()) == Some("table"))
            .ok_or_else(|| CommandError::from("Invalid Lexical JSON: missing table node"))?;
        
        let rows = table_node.get("children")
            .and_then(|r| r.as_array())
            .ok_or_else(|| CommandError::from("Invalid Lexical JSON: missing table children"))?;
        
        // Helper to parse timestamp range "mm:ss.mmm - mm:ss.mmm" or "hh:mm:ss.mmm - hh:mm:ss.mmm"
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
                // We expect at least 4 cells: #, Timestamp, Speaker, Text
                if cells.len() < 4 { continue; }

                // Use the defined extract_plain_text_from_lexical_value helper for simple fields
                let mut timestamp_buffer = String::new();
                extract_plain_text_from_lexical_value(&cells[1], &mut timestamp_buffer);
                let (start, end) = parse_ts_range(&timestamp_buffer);

                let mut speaker_buffer = String::new();
                extract_plain_text_from_lexical_value(&cells[2], &mut speaker_buffer);
                let speaker = speaker_buffer.trim().to_string();

                // For the Text column, we want to PRESERVE the Lexical structure if possible
                // So we take the cell's children and wrap them in a pseudo-root.
                let text_nodes = cells[3].get("children").cloned().unwrap_or(json!([]));
                let text_json = json!({
                    "root": {
                        "type": "root",
                        "children": text_nodes,
                        "direction": null,
                        "format": "",
                        "indent": 0,
                        "version": 1
                    }
                }).to_string();

                let seg = json!({
                    "start_time": start,
                    "end_time": end,
                    "speaker": speaker,
                    "text": text_json,
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
        font-family: 'Inter', Roboto, ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, \
        'Liberation Mono', 'Courier New', monospace; \
        font-size: 14px; \
        line-height: 1.5; \
    }\n\
    table { width: 100%; border-collapse: collapse; table-layout: fixed; }\n\
    td { vertical-align: top; text-align: left; padding: 4px; border: 1px solid #eee; }\n\
    p { margin: 0; padding: 0; text-align: left; }\n");
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
        let speaker_to_format = if raw_speaker.ends_with(':') {
            &raw_speaker[0..raw_speaker.len()-1]
        } else {
            raw_speaker
        };
        let speaker_display_with_colon = if speaker_to_format.chars().count() > 12 && speaker_to_format != "Unknown" {
            format!("{}:", speaker_to_format.chars().take(12).collect::<String>() + "...")
        } else {
            format!("{}:", speaker_to_format)
        };
        let raw_text_content = entry.get("text").and_then(Value::as_str).unwrap_or("");
        let segment_html_content = convert_lexical_or_plain_text_to_html(raw_text_content);

        match current_layout.as_str() {
            "Layout1" => { // | No | Timestamp | Speaker | Text |
                html_output.push_str("    <tr style=\"page-break-inside: avoid;\">\n");
                html_output.push_str(&format!("      <td style=\"vertical-align: top; text-align: left; padding: 4px; border: 1px solid #eee; font-size: 0.9em; color: #555;\"><p>{}</p></td>\n", segment_number));
                html_output.push_str(&format!("      <td style=\"vertical-align: top; text-align: left; padding: 4px; border: 1px solid #eee; font-size: 0.9em; color: #555;\"><p>{}</p></td>\n", encode_text(&timestamp_str)));
                html_output.push_str(&format!("      <td style=\"vertical-align: top; text-align: left; padding: 4px; border: 1px solid #eee; font-weight: bold;\"><p>{}</p></td>\n", encode_text(&speaker_display_with_colon)));
                html_output.push_str(&format!("      <td style=\"vertical-align: top; text-align: left; padding: 4px; border: 1px solid #eee;\">{}</td>\n", segment_html_content));
                html_output.push_str("    </tr>\n");
            }
            "Layout2" => { // | No | Timestamp | then | Speaker | Text | (Default)
                html_output.push_str("    <tr style=\"page-break-inside: avoid;\">\n");
                html_output.push_str(&format!("      <td style=\"vertical-align: top; text-align: left; padding: 4px; border: 1px solid #eee; font-size: 0.9em; color: #555;\"><p>{}</p></td>\n", segment_number));
                html_output.push_str(&format!("      <td style=\"vertical-align: top; text-align: left; padding: 4px; border: 1px solid #eee; font-size: 0.9em; color: #555;\"><p>{}</p></td>\n", encode_text(&timestamp_str)));
                html_output.push_str("    </tr>\n");
                html_output.push_str("    <tr style=\"page-break-inside: avoid;\">\n");
                html_output.push_str(&format!("      <td style=\"vertical-align: top; text-align: left; padding: 4px; border: 1px solid #eee; font-weight: bold;\"><p>{}</p></td>\n", encode_text(&speaker_display_with_colon)));
                html_output.push_str(&format!("      <td style=\"vertical-align: top; text-align: left; padding: 4px; border: 1px solid #eee;\">{}</td>\n", segment_html_content));
                html_output.push_str("    </tr>\n");
            }
            "Layout3" => { // | Timestamp Speaker | then | Text |
                html_output.push_str("    <tr style=\"page-break-inside: avoid;\">\n");
                html_output.push_str(&format!("      <td style=\"vertical-align: top; text-align: left; padding: 4px; border: 1px solid #eee; font-size: 0.9em; color: #555;\"><p>{} {}</p></td>\n", encode_text(&timestamp_str), encode_text(&speaker_display_with_colon)));
                html_output.push_str("    </tr>\n");
                html_output.push_str("    <tr style=\"page-break-inside: avoid;\">\n");
                html_output.push_str(&format!("      <td style=\"vertical-align: top; text-align: left; padding: 4px; border: 1px solid #eee;\">{}</td>\n", segment_html_content));
                html_output.push_str("    </tr>\n");
            }
            "Layout4" => { // | Speaker | Text |
                html_output.push_str("    <tr style=\"page-break-inside: avoid;\">\n");
                html_output.push_str(&format!("      <td style=\"vertical-align: top; text-align: left; padding: 4px; border: 1px solid #eee; font-weight: bold;\"><p>{}</p></td>\n", encode_text(&speaker_display_with_colon)));
                html_output.push_str(&format!("      <td style=\"vertical-align: top; text-align: left; padding: 4px; border: 1px solid #eee;\">{}</td>\n", segment_html_content));
                html_output.push_str("    </tr>\n");
            }
            "Layout5" => { // | Text |
                html_output.push_str("    <tr style=\"page-break-inside: avoid;\">\n");
                html_output.push_str(&format!("      <td style=\"vertical-align: top; text-align: left; padding: 4px; border: 1px solid #eee;\">{}</td>\n", segment_html_content));
                html_output.push_str("    </tr>\n");
            }
            _ => { // Fallback to Layout2 if layout_choice is unknown
                html_output.push_str("    <tr style=\"page-break-inside: avoid;\">\n");
                html_output.push_str(&format!("      <td style=\"vertical-align: top; text-align: left; padding: 4px; border: 1px solid #eee; font-size: 0.9em; color: #555;\"><p>{}</p></td>\n", segment_number));
                html_output.push_str(&format!("      <td style=\"vertical-align: top; text-align: left; padding: 4px; border: 1px solid #eee; font-size: 0.9em; color: #555;\"><p>{}</p></td>\n", encode_text(&timestamp_str)));
                html_output.push_str("    </tr>\n");
                html_output.push_str("    <tr style=\"page-break-inside: avoid;\">\n");
                html_output.push_str(&format!("      <td style=\"vertical-align: top; text-align: left; padding: 4px; border: 1px solid #eee; font-weight: bold;\"><p>{}</p></td>\n", encode_text(&speaker_display_with_colon)));
                html_output.push_str(&format!("      <td style=\"vertical-align: top; text-align: left; padding: 4px; border: 1px solid #eee;\">{}</td>\n", segment_html_content));
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

    let script_path = app_handle.path()
        .resolve("scripts/convert_with_pandoc.py", tauri::path::BaseDirectory::Resource)
        .map_err(|e| CommandError::from(format!("Failed to resolve pandoc script path: {}", e)))?;

    let mut pandoc_args = vec![
        temp_html_path.to_string_lossy().to_string(),
        output_path_str.clone(),
        "docx".to_string(),
    ];

    let lua_filter_path = app_handle.path()
        .resolve("scripts/docx_styles.lua", tauri::path::BaseDirectory::Resource)
        .ok();

    if let Some(lua_path) = lua_filter_path {
        pandoc_args.push("--lua-filter".to_string());
        pandoc_args.push(lua_path.to_string_lossy().to_string());
    }

    info!("[export_transcript_to_docx] Executing Pandoc script: {} {}", script_path.display(), pandoc_args.join(" "));

    let (mut rx, _child) = get_python_command(&app_handle)?
        .args(&[script_path.to_string_lossy().to_string()])
        .args(&pandoc_args)
        .spawn()
        .map_err(|e| {
            let msg = format!("Pandoc script execution failed: {}", e);
            error!("[export_transcript_to_docx] Pandoc script spawn failed: {}", e);
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
        return Err(CommandError::from(err_msg));
    }

    info!("[export_transcript_to_docx] Pandoc conversion successful.");

    debug!("[export_transcript_to_docx] Cleaning up temporary HTML file...");
    if let Err(e) = fs::remove_file(&temp_html_path) {
        warn!("[export_transcript_to_docx] Failed to delete temporary HTML file {}: {}", temp_html_path.display(), e);
    } else {
        debug!("[export_transcript_to_docx] Deleted temporary HTML file: {}", temp_html_path.display());
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
    exclude_speaker_names: Option<bool>,
) -> Result<String, CommandError> {
    info!("[export_transcript_to_srt] Exporting to SRT: {}", output_path_str);

    let segments: Vec<Segment> = serde_json::from_str(&segments_json_str)
        .map_err(|e| CommandError::from(format!("Failed to parse segments JSON for SRT: {}", e)))?;

    if segments.is_empty() {
        return Err(CommandError::from("No segments provided for SRT export."));
    }

    let exclude_speakers = exclude_speaker_names.unwrap_or(false);

    let mut srt_content = String::new();
    for (index, segment) in segments.iter().enumerate() { // Changed _index to index
        srt_content.push_str(&(index + 1).to_string()); // Use index here
        srt_content.push_str("\n");

        let start_ts = format_srt_timestamp(segment.start_time);
        let end_ts = format_srt_timestamp(segment.end_time);
        srt_content.push_str(&format!("{} --> {}\n", start_ts, end_ts));

        let plain_text = get_plain_text_for_srt(&segment.text);

        // Prepend speaker to text if speaker exists and is not empty AND not excluded
        let text_line = if !exclude_speakers {
            if let Some(speaker_name) = &segment.speaker {
                let trimmed_spk = speaker_name.trim();
                if !trimmed_spk.is_empty() {
                    let spk_with_colon = if trimmed_spk.ends_with(':') {
                        trimmed_spk.to_string()
                    } else {
                        format!("{}:", trimmed_spk)
                    };
                    format!("{} {}", spk_with_colon, plain_text)
                } else {
                    plain_text
                }
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

// Helper function to format seconds to HH:MM:SS.mmm for VTT
fn format_vtt_timestamp(seconds: f64) -> String {
    if seconds.is_nan() || seconds < 0.0 {
        return "00:00:00.000".to_string();
    }
    let total_ms = (seconds * 1000.0).round() as u64;
    let ms = total_ms % 1000;
    let total_s = total_ms / 1000;
    let s = total_s % 60;
    let total_m = total_s / 60;
    let m = total_m % 60;
    let h = total_m / 60;
    format!("{:02}:{:02}:{:02}.{:03}", h, m, s, ms)
}

// Helper function to ensure a color is in a format VTT/ASS players like (HEX)
// and handles "transparent" by returning an empty string.
fn ensure_vtt_color(color: &str) -> String {
    if color == "transparent" {
        return "".to_string();
    }
    let trimmed = color.trim();
    if trimmed.starts_with('#') {
        let hex = trimmed.trim_start_matches('#');
        if hex.len() == 3 {
            // Convert #RGB to #RRGGBB
            let r = &hex[0..1];
            let g = &hex[1..2];
            let b = &hex[2..3];
            return format!("#{}{}{}{}{}{}", r, r, g, g, b, b);
        }
        if hex.len() == 6 {
            return trimmed.to_string();
        }
    }
    // Handle rgb(r, g, b) format
    if let Ok(re) = Regex::new(r"rgb\s*\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)\s*\)") {
        if let Some(caps) = re.captures(trimmed) {
            if let (Ok(r), Ok(g), Ok(b)) = (
                caps[1].parse::<u8>(),
                caps[2].parse::<u8>(),
                caps[3].parse::<u8>(),
            ) {
                return format!("#{:02x}{:02x}{:02x}", r, g, b);
            }
        }
    }
    // Fallback for named colors or invalid formats
    match trimmed.to_lowercase().as_str() {
        "black" => "#000000".to_string(),
        "white" => "#ffffff".to_string(),
        "red" => "#ff0000".to_string(),
        "green" => "#00ff00".to_string(),
        "blue" => "#0000ff".to_string(),
        "yellow" => "#ffff00".to_string(),
        "magenta" | "fuchsia" => "#ff00ff".to_string(),
        "cyan" | "aqua" => "#00ffff".to_string(),
        _ => trimmed.to_string(),
    }
}

// Helper to convert Lexical color (hex like #RRGGBB or named) to ASS &HBBGGRR& (opaque)
fn lexical_color_to_ass_color(hex_color: &str) -> String {
    let sanitized = ensure_vtt_color(hex_color);
    let color = sanitized.trim_start_matches('#');
    if color.len() == 6 {
        let r = &color[0..2];
        let g = &color[2..4];
        let b = &color[4..6];
        format!("&H00{}{}{}&", b, g, r).to_uppercase() // ASS is BGR, opaque
    } else {
        // Fallback to white opaque
        "&H00FFFFFF&".to_string()
    }
}

// Helper function to convert Lexical JSON to VTT cue text with styling
fn lexical_to_vtt_cue_text(value: &Value, vtt_text_buffer: &mut String, used_classes: &mut std::collections::HashMap<String, String>) {
    if let Some(node_type) = value.get("type").and_then(|t| t.as_str()) {
        match node_type {
            "text" | "extended-text" => {
                if let Some(text_content) = value.get("text").and_then(|t| t.as_str()) {
                    let format_flags = value.get("format").and_then(|f| f.as_i64()).unwrap_or(0);
                    let style_str = value.get("style").and_then(|s| s.as_str()).unwrap_or("");

                    let mut prefix_tags = String::new();
                    let mut suffix_tags = String::new();

                    let mut has_strikethrough = (format_flags & IS_STRIKETHROUGH) != 0;
                    let mut color_found: Option<String> = None;
                    let mut has_highlight = (format_flags & IS_HIGHLIGHT) != 0;

                    // Parse styles - strictly for text color and decorations
                    if !style_str.is_empty() {
                        for part in style_str.split(';') {
                            let part_trimmed = part.trim();
                            if part_trimmed.starts_with("color:") {
                                let val = part_trimmed.trim_start_matches("color:").trim();
                                if !val.is_empty() { 
                                    let sanitized = ensure_vtt_color(val);
                                    if !sanitized.is_empty() {
                                        color_found = Some(sanitized);
                                    }
                                }
                            } else if part_trimmed.starts_with("background-color:") {
                                let val = part_trimmed.trim_start_matches("background-color:").trim();
                                if !val.is_empty() && val != "transparent" {
                                    has_highlight = true;
                                }
                            } else if part_trimmed.starts_with("text-decoration:") {
                                if part_trimmed.contains("line-through") {
                                    has_strikethrough = true;
                                }
                            }
                        }
                    }

                    // Apply color if found and not white/default.
                    // Special logic inspired by ASS export: If highlight is present, ignore Black text color 
                    // to avoid invisible text on dark video (since we don't export background box for VTT span).
                    if let Some(color) = color_found {
                        let is_white = color.eq_ignore_ascii_case("#FFFFFF") || color.eq_ignore_ascii_case("#FFF") || color.eq_ignore_ascii_case("white");
                        let is_black = color.eq_ignore_ascii_case("#000000") || color.eq_ignore_ascii_case("#000") || color.eq_ignore_ascii_case("black");
                        
                        if !is_white && (!has_highlight || !is_black) {
                            // Create a safe alphanumeric class name
                            // Strip # if present to avoid double underscores
                            let safe_suffix = color.trim_start_matches('#').chars()
                                .map(|c| if c.is_alphanumeric() { c } else { '_' })
                                .collect::<String>();
                            let class_name = format!("c_{}", safe_suffix);

                            prefix_tags.push_str(&format!("<c.{}>", class_name));
                            suffix_tags.insert_str(0, "</c>");
                            used_classes.insert(class_name, color);
                        }
                    }

                    if format_flags & IS_BOLD != 0 { prefix_tags.push_str("<b>"); suffix_tags.insert_str(0, "</b>"); }
                    if format_flags & IS_ITALIC != 0 { prefix_tags.push_str("<i>"); suffix_tags.insert_str(0, "</i>"); }
                    if format_flags & IS_UNDERLINE != 0 { prefix_tags.push_str("<u>"); suffix_tags.insert_str(0, "</u>"); }
                    if has_strikethrough { 
                        prefix_tags.push_str("<c.s>");
                        suffix_tags.insert_str(0, "</c>");
                        used_classes.insert("s".to_string(), "line-through".to_string()); // Value not strictly used for 's' but keeps map consistent
                    }

                    vtt_text_buffer.push_str(&prefix_tags);
                    vtt_text_buffer.push_str(&encode_text(text_content));
                    vtt_text_buffer.push_str(&suffix_tags);
                }
            }

            "linebreak" => {
                vtt_text_buffer.push_str("\n");
            }
            "paragraph" | "heading" | "list" | "listitem" | "quote" | "link" | "table" | "tablecell" | "tablerow" => {
                if let Some(children) = value.get("children").and_then(|c| c.as_array()) {
                    for child in children {
                        lexical_to_vtt_cue_text(child, vtt_text_buffer, used_classes);
                    }
                }
            }
            _ => {
                if let Some(children) = value.get("children").and_then(|c| c.as_array()) {
                    for child in children {
                        lexical_to_vtt_cue_text(child, vtt_text_buffer, used_classes);
                    }
                }
            }
        }
    } else if let Some(children) = value.get("root").and_then(|r| r.get("children")).and_then(|c| c.as_array()) {
        for (i, child) in children.iter().enumerate() {
            lexical_to_vtt_cue_text(child, vtt_text_buffer, used_classes);
            if i < children.len() - 1 {
                if let Some(child_node_type) = child.get("type").and_then(|t| t.as_str()) {
                    if child_node_type == "paragraph" && !vtt_text_buffer.ends_with('\n') {
                        vtt_text_buffer.push_str("\n");
                    }
                }
            }
        }
    } else if value.is_string() {
         vtt_text_buffer.push_str(value.as_str().unwrap_or(""));
    }
}

fn get_vtt_cue_text_from_lexical_string(text_content: &str, used_classes: &mut std::collections::HashMap<String, String>) -> String {
    match serde_json::from_str::<Value>(text_content) {
        Ok(parsed_json) => {
            if parsed_json.get("root").and_then(|r| r.get("children")).is_some() {
                let mut buffer = String::new();
                lexical_to_vtt_cue_text(&parsed_json, &mut buffer, used_classes);
                return buffer;
            }
            if parsed_json.is_string() {
                return parsed_json.as_str().unwrap_or("").to_string();
            }
            text_content.to_string()
        }
        Err(_) => {
            text_content.to_string()
        }
    }
}


#[tauri::command]
pub async fn export_transcript_to_vtt(
    _app_handle: AppHandle,
    output_path_str: String,
    segments_json_str: String,
    exclude_speaker_names: Option<bool>,
) -> Result<String, CommandError> {
    info!("[export_transcript_to_vtt] Exporting to VTT with styling: {}", output_path_str);

    let segments: Vec<Segment> = serde_json::from_str(&segments_json_str)
        .map_err(|e| CommandError::from(format!("Failed to parse segments JSON for VTT: {}", e)))?;

    if segments.is_empty() {
        return Err(CommandError::from("No segments provided for VTT export."));
    }

    let exclude_speakers = exclude_speaker_names.unwrap_or(false);

    // Two-pass approach for VTT:
    // 1. Generate cue texts and collect all used classes (strikethrough and colors)
    // 2. Build the final VTT with a STYLE block containing only the needed classes.

    let mut used_classes = std::collections::HashMap::new();
    let mut cue_texts = Vec::new();

    for segment in segments.iter() {
        let cue_text = get_vtt_cue_text_from_lexical_string(&segment.text, &mut used_classes);
        cue_texts.push(cue_text);
    }

    let mut vtt_content = String::from("WEBVTT\n\n");

    if !used_classes.is_empty() {
        vtt_content.push_str("STYLE\n");
        // Sort keys for deterministic output
        let mut sorted_keys: Vec<_> = used_classes.keys().collect();
        sorted_keys.sort();

        for class in sorted_keys {
            if class == "s" {
                vtt_content.push_str("::cue(.s) { text-decoration: line-through; }\n");
            } else if class.starts_with("c_") {
                if let Some(css_color) = used_classes.get(class) {
                    vtt_content.push_str(&format!("::cue(.{}) {{ color: {}; }}\n", class, css_color));
                }
            }
        }
        vtt_content.push('\n');
    }

    for (index, segment) in segments.iter().enumerate() {
        let start_ts = format_vtt_timestamp(segment.start_time);
        let end_ts = format_vtt_timestamp(segment.end_time);
        vtt_content.push_str(&format!("{} --> {}\n", start_ts, end_ts));

        let cue_text = &cue_texts[index];

        let text_line = if !exclude_speakers {
            if let Some(speaker_name) = &segment.speaker {
                let trimmed_spk = speaker_name.trim();
                if !trimmed_spk.is_empty() {
                    let spk_with_colon = if trimmed_spk.ends_with(':') {
                        trimmed_spk.to_string()
                    } else {
                        format!("{}:", trimmed_spk)
                    };
                    format!("{} {}", spk_with_colon, cue_text)
                } else {
                    cue_text.clone()
                }
            } else {
                cue_text.clone()
            }
        } else {
            cue_text.clone()
        };
        vtt_content.push_str(&text_line);
        vtt_content.push_str("\n\n");
    }

    fs::write(&output_path_str, vtt_content)
        .map_err(|e| CommandError::from(format!("Failed to write VTT file {}: {}", output_path_str, e)))?;

    info!("[export_transcript_to_vtt] VTT export successful to {}", output_path_str);
    Ok(output_path_str)
}

// Helper function to convert Lexical JSON to Markdown text with bold/italic
fn lexical_to_markdown_text_node(node: &Value, buffer: &mut String) {
    if let Some(node_type) = node.get("type").and_then(|t| t.as_str()) {
        match node_type {
            "text" | "extended-text" => {
                if let Some(text_content) = node.get("text").and_then(|t| t.as_str()) {
                    if text_content.trim().is_empty() && buffer.ends_with(' ') {
                        // Avoid adding multiple spaces if text is just whitespace after a space
                    } else if text_content.trim().is_empty() && !buffer.is_empty() && !buffer.ends_with('\n') {
                        buffer.push(' '); // Add a space for empty text nodes if not at start of a line.
                    } else {
                        let format_flags = node.get("format").and_then(|f| f.as_i64()).unwrap_or(0);
                        let is_bold = (format_flags & IS_BOLD) != 0;
                        let is_italic = (format_flags & IS_ITALIC) != 0;

                        let mut prefix = String::new();
                        let mut suffix = String::new();

                        if is_bold && is_italic {
                            prefix.push_str("***");
                            suffix.push_str("***");
                        } else if is_bold {
                            prefix.push_str("**");
                            suffix.push_str("**");
                        } else if is_italic {
                            prefix.push_str("*");
                            suffix.push_str("*");
                        }

                        // Escape Markdown special characters in the text_content itself
                        let escaped_text = text_content
                            .replace("*", "\\*")
                            .replace("_", "\\_")
                            .replace("`", "\\`")
                            .replace("[", "\\[")
                            .replace("]", "\\]")
                            .replace("#", "\\#");

                        buffer.push_str(&prefix);
                        buffer.push_str(&escaped_text);
                        buffer.push_str(&suffix);
                    }
                }
            }
            "linebreak" => {
                buffer.push_str("\n");
            }
            "paragraph" | "heading" | "listitem" | "quote" => { // Treat these as block elements
                if !buffer.is_empty() && !buffer.ends_with("\n\n") && !buffer.ends_with("\n") { // Ensure space before new block unless already newlined
                     buffer.push_str("\n"); // Start new paragraph on a new line
                }
                if let Some(children) = node.get("children").and_then(|c| c.as_array()) {
                    for child in children {
                        lexical_to_markdown_text_node(child, buffer);
                    }
                }
                buffer.push_str("\n"); // End paragraph with a newline, will become double with next paragraph's start
            }
            "link" => { // Format as Markdown link: [text](url)
                let url = node.get("url").and_then(|u| u.as_str()).unwrap_or("");
                buffer.push_str("[");
                if let Some(children) = node.get("children").and_then(|c| c.as_array()) {
                    for child in children {
                        lexical_to_markdown_text_node(child, buffer); // Process link text
                    }
                }
                buffer.push_str(&format!("]({})", encode_text(url))); // encode_text for URL safety
            }
            // Other block types like list, table, tablecell, tablerow are not directly translated to simple markdown text here.
            // They would require more complex handling if their structure is to be preserved in Markdown.
            // For now, just recurse through children to extract any text.
            "list" | "table" | "tablerow" | "tablecell" => {
                 if let Some(children) = node.get("children").and_then(|c| c.as_array()) {
                    for child in children {
                        lexical_to_markdown_text_node(child, buffer);
                         if node_type == "tablecell" { buffer.push_str(" "); } // Add space between cell contents
                    }
                }
                if node_type == "tablerow" { buffer.push_str("\n");} // Newline after each row
            }
            _ => { // Generic fallback for other unknown node types
                if let Some(children) = node.get("children").and_then(|c| c.as_array()) { // Corrected: value -> node
                    for child in children {
                        lexical_to_markdown_text_node(child, buffer);
                    }
                }
            }
        }
    }
}

fn get_markdown_text_from_lexical_string(text_content: &str) -> String {
    match serde_json::from_str::<Value>(text_content) {
        Ok(parsed_json) => {
            if parsed_json.get("root").and_then(|r| r.get("children")).is_some() {
                let mut buffer = String::new();
                if let Some(children) = parsed_json.get("root").and_then(|r| r.get("children")).and_then(|c| c.as_array()) {
                    for (i, child_node) in children.iter().enumerate() {
                        lexical_to_markdown_text_node(child_node, &mut buffer);
                        if i < children.len() - 1 { // Add double newline between top-level blocks from Lexical root
                           if !buffer.ends_with("\n\n") {
                                if buffer.ends_with("\n") { buffer.push_str("\n"); }
                                else { buffer.push_str("\n\n"); }
                           }
                        }
                    }
                }
                // Trim trailing newlines but try to preserve internal structure like double newlines between paragraphs
                let mut result = buffer.as_str();
                while result.ends_with('\n') {
                    result = &result[0..result.len()-1];
                }
                result.to_string()

            } else if parsed_json.is_string() { // JSON string value (already plain)
                parsed_json.as_str().unwrap_or("").to_string()
            } else { // Other JSON, not Lexical root or plain string
                text_content.to_string()
            }
        }
        Err(_) => { // Not valid JSON, assume it's already plain text
            text_content.to_string()
        }
    }
}


#[tauri::command]
pub async fn export_transcript_to_markdown(
    _app_handle: AppHandle,
    output_path_str: String,
    segments_json_str: String,
    layout_choice: Option<String>,
) -> Result<String, CommandError> {
    let current_layout = layout_choice.unwrap_or_else(|| "Layout2".to_string());
    info!(
        "[export_transcript_to_markdown] Exporting to Markdown: {}, Layout: {}",
        output_path_str, current_layout
    );

    let segments: Vec<Segment> = serde_json::from_str(&segments_json_str)
        .map_err(|e| CommandError::from(format!("Failed to parse segments JSON for Markdown: {}", e)))?;

    if segments.is_empty() {
        return Err(CommandError::from("No segments provided for Markdown export."));
    }

    let mut md_content = String::new();

    // Re-use timestamp formatting, SRT's HH:MM:SS,mmm is fine for Md info
    // Or define a simpler one if preferred for Markdown. Using SRT's for now.
    // fn format_markdown_timestamp(seconds: f64) -> String { format_srt_timestamp(seconds) }

    if current_layout == "Layout1" {
        md_content.push_str("| # | Timestamp | Speaker | Text |\n");
        md_content.push_str("|---|-----------|---------|------|\n");
    } else if current_layout == "Layout4" {
        md_content.push_str("| Speaker | Text |\n");
        md_content.push_str("|---------|------|\n");
    }

    for (index, segment) in segments.iter().enumerate() { // Changed _index to index
        let segment_number = index + 1; // Use index here for numbering
        // Using srt_timestamp for consistency, but could be simplified for MD
        let timestamp_str = format!("{} - {}", format_srt_timestamp(segment.start_time), format_srt_timestamp(segment.end_time));
        let raw_speaker = segment.speaker.as_deref().unwrap_or("Unknown");

        let speaker_to_format = if raw_speaker.ends_with(':') {
            &raw_speaker[0..raw_speaker.len()-1]
        } else {
            raw_speaker
        };

        let speaker_display_with_colon = if speaker_to_format.chars().count() > 12 && speaker_to_format != "Unknown" {
            format!("{}:", speaker_to_format.chars().take(12).collect::<String>() + "...")
        } else {
            format!("{}:", speaker_to_format)
        };

        let markdown_text = get_markdown_text_from_lexical_string(&segment.text);

        match current_layout.as_str() {
            "Layout1" => { // | # | Timestamp | Speaker | Text |
                // For Markdown tables, internal newlines in content are tricky.
                // Replacing with space or <br> (if renderer supports HTML) are options.
                // Here, replacing with space for broader compatibility.
                let table_cell_text = markdown_text.replace("\n", " ");
                md_content.push_str(&format!(
                    "| {} | {} | {} | {} |\n",
                    segment_number,
                    encode_text(&timestamp_str),
                    encode_text(&speaker_display_with_colon),
                    table_cell_text // Already contains Markdown, no further encode_text
                ));
            }
            "Layout2" => { // | No | Timestamp | then | Speaker | Text |
                if segment_number > 1 { md_content.push_str("\n"); } // Use segment_number for condition
                md_content.push_str(&format!("**Segment {}** - {}\n\n", segment_number, encode_text(&timestamp_str)));
                md_content.push_str(&format!("**{}** {}\n", encode_text(&speaker_display_with_colon), markdown_text));
            }
            "Layout3" => { // | Timestamp Speaker | then | Text |
                if segment_number > 1 { md_content.push_str("\n"); } // Use segment_number for condition
                md_content.push_str(&format!("**{} {}**\n\n", encode_text(&timestamp_str), encode_text(&speaker_display_with_colon)));
                md_content.push_str(&format!("{}\n", markdown_text));
            }
            "Layout4" => { // | Speaker | Text |
                let table_cell_text = markdown_text.replace("\n", " ");
                 md_content.push_str(&format!(
                    "| {} | {} |\n",
                    encode_text(&speaker_display_with_colon),
                    table_cell_text
                ));
            }
            "Layout5" => { // | Text |
                if segment_number > 1 { md_content.push_str("\n"); } // Use segment_number for condition
                md_content.push_str(&format!("{}\n", markdown_text));
            }
            _ => { // Fallback to Layout2
                if segment_number > 1 { md_content.push_str("\n"); } // Use segment_number for condition
                md_content.push_str(&format!("**Segment {}** - {}\n\n", segment_number, encode_text(&timestamp_str)));
                md_content.push_str(&format!("**{}** {}\n", encode_text(&speaker_display_with_colon), markdown_text));
            }
        }
        if current_layout != "Layout1" && current_layout != "Layout4" {
             md_content.push_str("\n");
        }
    }

    fs::write(&output_path_str, md_content)
        .map_err(|e| CommandError::from(format!("Failed to write Markdown file {}: {}", output_path_str, e)))?;

    info!("[export_transcript_to_markdown] Markdown export successful to {}", output_path_str);
    Ok(output_path_str)
}

// --- ASS Export Implementation ---

// Helper function to format seconds to H:MM:SS.ss for ASS
fn format_ass_timestamp(seconds: f64) -> String {
    if seconds.is_nan() || seconds < 0.0 {
        return "0:00:00.00".to_string();
    }
    let total_centiseconds = (seconds * 100.0).round() as u64;
    let cs = total_centiseconds % 100;
    let total_s = total_centiseconds / 100;
    let s = total_s % 60;
    let total_m = total_s / 60;
    let m = total_m % 60;
    let h = total_m / 60;
    format!("{}:{:02}:{:02}.{:02}", h, m, s, cs)
}

fn lexical_node_to_ass_tags(node: &Value, ass_buffer: &mut String) {

    if let Some(node_type) = node.get("type").and_then(|t| t.as_str()) {

        match node_type {

            "text" | "extended-text" => {

                if let Some(text_content) = node.get("text").and_then(|t| t.as_str()) {

                    let format_flags = node.get("format").and_then(|f| f.as_i64()).unwrap_or(0);

                    let style_str = node.get("style").and_then(|s| s.as_str()).unwrap_or("");



                    let mut has_highlight = (format_flags & IS_HIGHLIGHT) != 0;

                    let mut font_color: Option<&str> = None;



                    let is_bold = (format_flags & IS_BOLD) != 0;

                    let is_italic = (format_flags & IS_ITALIC) != 0;

                    let mut is_underline = (format_flags & IS_UNDERLINE) != 0;

                    let mut is_strikethrough = (format_flags & IS_STRIKETHROUGH) != 0;



                    // Determine if highlight is present and extract font color and text decorations

                    for part in style_str.split(';') {

                        let p = part.trim();

                        if p.starts_with("color:") {

                            font_color = Some(p.trim_start_matches("color:").trim());

                        } else if p.starts_with("background-color:") {

                            let bg = p.trim_start_matches("background-color:").trim();

                            if bg != "transparent" && !bg.is_empty() {

                                has_highlight = true;

                            }

                        } else if p.starts_with("text-decoration:") {

                            let decoration = p.trim_start_matches("text-decoration:").trim();

                            if decoration.contains("line-through") {

                                is_strikethrough = true;

                            }

                            if decoration.contains("underline") {

                                is_underline = true;

                            }

                        }

                    }



                    let mut open_tags = String::from("{");

                    let mut has_tags = false;



                    if is_bold { open_tags.push_str("\\b1"); has_tags = true; }

                    if is_italic { open_tags.push_str("\\i1"); has_tags = true; }

                    if is_underline { open_tags.push_str("\\u1"); has_tags = true; }

                    if is_strikethrough { open_tags.push_str("\\s1"); has_tags = true; }



                    // Apply font color if present.
                    if let Some(color) = font_color {
                        if color != "transparent" && !color.is_empty() {
                            let ass_color = lexical_color_to_ass_color(color);
                            // We avoid applying black color if a highlight is present
                            // because we don't export the highlight background, 
                            // and black text on a dark video is unreadable.
                            let is_black = ass_color == "&H00000000&" || ass_color == "&H000000&" || 
                                           color == "#000000" || color == "#000" || color == "black";
                            
                            if !has_highlight || !is_black {
                                open_tags.push_str(&format!("\\1c{}", ass_color));
                                has_tags = true;
                            }
                        }
                    }
                    open_tags.push('}');



                    if has_tags {

                        ass_buffer.push_str(&open_tags);

                    }



                                        // ASS text should not contain literal curly braces unless they are part of tags.



                                        let mut ass_safe_text = text_content.replace("{", "\\{").replace("}", "\\}").replace("\r\n", "\\N").replace("\n", "\\N");



                    



                                        // Preserve multiple spaces by converting them to \h (hard space)



                                        // We only do this if there are 2 or more consecutive spaces.



                                        if ass_safe_text.contains("  ") {



                                            if let Ok(re) = Regex::new(r" {2,}") {



                                                ass_safe_text = re.replace_all(&ass_safe_text, |caps: &regex::Captures| {



                                                    "\\h".repeat(caps[0].len())



                                                }).to_string();



                                            }



                                        }



                    



                                        ass_buffer.push_str(&ass_safe_text);



                    // Reset to default style to close all tags at once

                    if has_tags {

                        ass_buffer.push_str("{\\r}");

                    }

                }

            }

            "linebreak" => {

                ass_buffer.push_str("\\N");

            }

                        "paragraph" | "heading" | "listitem" | "quote" => { // Block elements

                            if let Some(children) = node.get("children").and_then(|c| c.as_array()) {

                                for child in children {

                                    lexical_node_to_ass_tags(child, ass_buffer);

                                }

                            }

                            // Ensure block elements end with a newline.

                            // We removed the 'ends_with("\\N")' check to allow consecutive breaks (e.g. empty paragraphs).

                            if !ass_buffer.is_empty() {

                                ass_buffer.push_str("\\N");

                            }

                        }

                        "link" | "autolink" | "list" | "table" | "tablerow" | "tablecell" => { // Containers

                            if let Some(children) = node.get("children").and_then(|c| c.as_array()) {

                                for child in children {

                                    lexical_node_to_ass_tags(child, ass_buffer);

                                    if node_type == "tablecell" { ass_buffer.push_str(" "); }

                                }

                            }

                            if node_type == "tablerow" || node_type == "list" {

                                if !ass_buffer.is_empty() {

                                    ass_buffer.push_str("\\N");

                                }

                            }

                        }

            _ => {

                if let Some(children) = node.get("children").and_then(|c| c.as_array()) {

                    for child in children {

                        lexical_node_to_ass_tags(child, ass_buffer);

                    }

                }

            }

        }

    }

}



fn get_ass_dialogue_line_from_lexical_string(text_content: &str) -> String {

    match serde_json::from_str::<Value>(text_content) {

        Ok(parsed_json) => {

            if parsed_json.get("root").and_then(|r| r.get("children")).is_some() {

                let mut buffer = String::new();

                 if let Some(children) = parsed_json.get("root").and_then(|r| r.get("children")).and_then(|c| c.as_array()) {

                    for child_node in children {

                        lexical_node_to_ass_tags(child_node, &mut buffer);

                    }

                }

                // Trim trailing \N

                let mut result = buffer.as_str();

                while result.ends_with("\\N") {

                    result = &result[0..result.len()-2];

                }

                return result.to_string();

            }

            if parsed_json.is_string() { // JSON string value (already plain)

                return parsed_json.as_str().unwrap_or("").replace("\r\n", "\\N").replace("\n", "\\N");

            }

            text_content.replace("\r\n", "\\N").replace("\n", "\\N")

        }

        Err(_) => { // Not valid JSON, assume it's already plain text

            text_content.replace("\r\n", "\\N").replace("\n", "\\N")

        }

    }

}




#[tauri::command]
pub async fn export_transcript_to_ass(
    _app_handle: AppHandle,
    output_path_str: String,
    segments_json_str: String,
    exclude_speaker_names: Option<bool>,
) -> Result<String, CommandError> {
    info!("[export_transcript_to_ass] Exporting to ASS: {}", output_path_str);

    let segments: Vec<Segment> = serde_json::from_str(&segments_json_str)
        .map_err(|e| CommandError::from(format!("Failed to parse segments JSON for ASS: {}", e)))?;

    if segments.is_empty() {
        return Err(CommandError::from("No segments provided for ASS export."));
    }

    let exclude_speakers = exclude_speaker_names.unwrap_or(false);

    let mut ass_content = String::new();

    // [Script Info]
    ass_content.push_str("[Script Info]\n");
    ass_content.push_str("; Script generated by Harvey\n");
    ass_content.push_str("Title: Transcript Export\n");
    ass_content.push_str("ScriptType: v4.00+\n");
    ass_content.push_str("PlayResX: 384\n"); // Common default, can be adjusted
    ass_content.push_str("PlayResY: 288\n"); // Common default
    ass_content.push_str("WrapStyle: 0\n"); // Smart wrapping, respecting explicit line breaks
    ass_content.push_str("ScaledBorderAndShadow: yes\n");
    ass_content.push_str("\n");

    // [V4+ Styles]
    ass_content.push_str("[V4+ Styles]\n");
    ass_content.push_str("Format: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding\n");
    // Default style: Arial (standard proportional font), 20pt, White text, Yellow secondary (for karaoke), Black outline, Transparent shadow box
    // Opaque box for background: BorderStyle=3, Outline=border width, BackColour=highlight color
    ass_content.push_str("Style: Default,Arial,20,&H00FFFFFF,&H0000FFFF,&H00000000,&H00000000,0,0,0,0,100,100,0,0,1,1.5,0,2,10,10,10,1\n");

    // Highlight styles removed for simpler export.
    let _styles_map: std::collections::HashMap<String, String> = std::collections::HashMap::new();


    // [Events]
    ass_content.push_str("[Events]\n");
    ass_content.push_str("Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text\n");

    for (_index, segment) in segments.iter().enumerate() { // Changed to _index
        let start_ts = format_ass_timestamp(segment.start_time);
        let end_ts = format_ass_timestamp(segment.end_time);

        let speaker_name = segment.speaker.as_deref().unwrap_or("").trim();
        let dialogue_text_raw = get_ass_dialogue_line_from_lexical_string(&segment.text);

        // Prepend speaker to dialogue text if speaker name is not empty AND not excluded
        let final_text = if !exclude_speakers && !speaker_name.is_empty() {
            let spk_with_colon = if speaker_name.ends_with(':') {
                speaker_name.to_string()
            } else {
                format!("{}:", speaker_name)
            };
            format!("{} {}", spk_with_colon, dialogue_text_raw)
        } else {
            dialogue_text_raw
        };

        ass_content.push_str(&format!(
            "Dialogue: 0,{},{},Default,{},0,0,0,,{}\n",
            start_ts, end_ts, encode_text(speaker_name), final_text // Speaker name also in Name field
        ));
    }

    fs::write(&output_path_str, ass_content)
        .map_err(|e| CommandError::from(format!("Failed to write ASS file {}: {}", output_path_str, e)))?;

    info!("[export_transcript_to_ass] ASS export successful to {}", output_path_str);
    Ok(output_path_str)
}

// --- Generic Document Export Commands ---

#[tauri::command]
pub async fn export_document_to_docx<R: Runtime>(
    app_handle: AppHandle<R>,
    document_path_str: String,
    output_path_str: String,
) -> Result<String, CommandError> {
    info!(
        "[export_document_to_docx] Starting export from JSON: {}, Target DOCX: {}",
        document_path_str, output_path_str
    );

    let source_path = PathBuf::from(&document_path_str);
    if !source_path.exists() || !source_path.is_file() {
        let msg = format!("Document JSON file not found: {}", document_path_str);
        error!("[export_document_to_docx] {}", msg);
        return Err(CommandError::from(msg));
    }

     let _base_dir = source_path
         .parent()
         .and_then(|p| p.parent()) // .../documents/
         .and_then(|p| p.parent()) // .../data/
         .and_then(|p| p.parent()) // .../Harvery_Data/
         .ok_or_else(|| {
             CommandError::from(format!("Could not determine project base directory from document path: {}", document_path_str))
         })?;
    
    // We can just use the document's directory to find the 'files' dir context if needed,
    // but for get_unique_temp_path_for_conversion, we need the "project root" ideally.
    // However, get_unique_temp_path_for_conversion takes `base_dir`.
    // Let's assume `base_dir` found above is roughly correct or sufficient to find `files/documents/.tmp`.
    // Actually, ensure_base_asset_dirs expects the `files` parent (Project Root).
    // Let's try to be robust.

    let output_path = PathBuf::from(&output_path_str);
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).map_err(|e| CommandError::from(format!("Failed to create output directory {}: {}", parent.display(), e)))?;
    }

    let json_content = fs::read_to_string(&source_path)?;
    let json_value: Value = serde_json::from_str(&json_content)
        .map_err(|e| CommandError::from(format!("Failed to parse document JSON: {}", e)))?;

    // Convert Lexical JSON to HTML
    let body_html = lexical_value_to_html(&json_value);

    let mut html_output = String::new();
    html_output.push_str("<!DOCTYPE html>\n");
    html_output.push_str("<html><head><meta charset=\"utf-8\"/><style>\n");
    html_output.push_str("body { \
        font-family: 'Inter', Roboto, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', 'Helvetica Neue', Arial, sans-serif; \
        font-size: 11pt; \
        line-height: 1.5; \
    }\n");
    html_output.push_str("</style></head><body>\n");
    html_output.push_str(&body_html);
    html_output.push_str("</body></html>\n");

    // Use a simpler base dir for temp file if the elaborate parent traversal is risky.
    // We just need A directory. source_path parent is safe.
    let _safe_base_dir = source_path.parent().unwrap_or(&source_path); 
    // But get_unique_temp_path_for_conversion builds path: base_dir/files/documents/.tmp/...
    // So if safe_base_dir is .../documents, joining files/documents/.tmp will fail.
    // We need the Project Root.
    // Assuming standard structure: Project/files/documents/doc.json
    let project_root = source_path
        .parent().and_then(|p| p.parent()).and_then(|p| p.parent())
        .ok_or(CommandError::from("Invalid project structure"))?;

    let stem = source_path.file_stem().and_then(|s| s.to_str()).unwrap_or("document_export");
    let temp_html_path = get_unique_temp_path_for_conversion(project_root, stem, "html")?;
    
    debug!("[export_document_to_docx] Writing generated HTML to temp file: {}", temp_html_path.display());
    fs::write(&temp_html_path, &html_output)?;

    let script_path = app_handle.path()
        .resolve("scripts/convert_with_pandoc.py", tauri::path::BaseDirectory::Resource)
        .map_err(|e| CommandError::from(format!("Failed to resolve pandoc script path: {}", e)))?;

    let mut pandoc_args = vec![
        temp_html_path.to_string_lossy().to_string(),
        output_path_str.clone(),
        "docx".to_string(),
    ];

    let lua_filter_path = app_handle.path()
        .resolve("scripts/docx_styles.lua", tauri::path::BaseDirectory::Resource)
        .ok();

    if let Some(lua_path) = lua_filter_path {
        pandoc_args.push("--lua-filter".to_string());
        pandoc_args.push(lua_path.to_string_lossy().to_string());
    }

    info!("[export_document_to_docx] Executing Pandoc script: {} {}", script_path.display(), pandoc_args.join(" "));

    let (mut rx, _child) = get_python_command(&app_handle)?
        .args(&[script_path.to_string_lossy().to_string()])
        .args(&pandoc_args)
        .spawn()
        .map_err(|e| {
            let msg = format!("Pandoc script execution failed: {}", e);
            error!("[export_document_to_docx] Pandoc script spawn failed: {}", e);
            CommandError::from(msg)
        })?;

    let mut pandoc_stderr = String::new();
    let mut exit_code = None;

    while let Some(event) = rx.recv().await {
        match event {
            CommandEvent::Stderr(line) => {
                let line_str = String::from_utf8_lossy(&line);
                pandoc_stderr.push_str(&line_str);
            },
            CommandEvent::Terminated(payload) => {
                exit_code = payload.code;
                break;
            }
            _ => { }
        }
    }

    if exit_code != Some(0) {
        let err_msg = format!("Pandoc conversion failed (exit code {:?}). Stderr:\n{}", exit_code, pandoc_stderr);
        error!("[export_document_to_docx] {}", err_msg);
        let _ = fs::remove_file(&temp_html_path);
        return Err(CommandError::from(err_msg));
    }

    let _ = fs::remove_file(&temp_html_path);
    
    info!("[export_document_to_docx] Export successful. DOCX saved to {}", output_path.display());
    Ok(output_path.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn export_document_to_markdown(
    _app_handle: AppHandle,
    document_path_str: String,
    output_path_str: String,
) -> Result<String, CommandError> {
    info!("[export_document_to_markdown] Exporting to Markdown: {}", output_path_str);

    let source_path = PathBuf::from(&document_path_str);
    let json_content = fs::read_to_string(&source_path)
        .map_err(|e| CommandError::from(format!("Failed to read document file: {}", e)))?;

    let md_content = get_markdown_text_from_lexical_string(&json_content);

    fs::write(&output_path_str, md_content)
        .map_err(|e| CommandError::from(format!("Failed to write Markdown file {}: {}", output_path_str, e)))?;

    info!("[export_document_to_markdown] Markdown export successful to {}", output_path_str);
    Ok(output_path_str)
}

#[tauri::command]
pub async fn export_document_to_txt(
    _app_handle: AppHandle,
    document_path_str: String,
    output_path_str: String,
) -> Result<String, CommandError> {
    info!("[export_document_to_txt] Exporting to TXT: {}", output_path_str);

    let source_path = PathBuf::from(&document_path_str);
    let json_content = fs::read_to_string(&source_path)
        .map_err(|e| CommandError::from(format!("Failed to read document file: {}", e)))?;

    let mut txt_content = String::new();
    match serde_json::from_str::<Value>(&json_content) {
        Ok(parsed_json) => {
            if parsed_json.get("root").is_some() {
                 extract_plain_text_from_lexical_value(&parsed_json, &mut txt_content);
            } else if parsed_json.is_string() {
                txt_content = parsed_json.as_str().unwrap_or("").to_string();
            } else {
                 txt_content = json_content; // Fallback
            }
        }
        Err(_) => {
            txt_content = json_content; // Fallback
        }
    }

    fs::write(&output_path_str, txt_content.trim())
        .map_err(|e| CommandError::from(format!("Failed to write TXT file {}: {}", output_path_str, e)))?;

    info!("[export_document_to_txt] TXT export successful to {}", output_path_str);
    Ok(output_path_str)
}