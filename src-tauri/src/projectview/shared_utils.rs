// src-tauri/src/projectview/shared_utils.rs
use super::shared_types::*;
use crate::welcome::config::CommandError;
use log::{debug, error, info, warn};
use std::{
    fs::{self, File},
    io::{BufWriter, Write},
    path::{Path, PathBuf, StripPrefixError},
};


impl From<StripPrefixError> for CommandError {
    fn from(err: StripPrefixError) -> Self {
        CommandError::from(format!("Path prefix error: {}", err))
    }
}


pub fn get_project_xml_path_from_item(item_path: &Path) -> Result<PathBuf, CommandError> {
    info!( "[get_project_xml_path] ENTER Function. Starting search from item: {}", item_path.display() );
    let mut current_path = item_path;
    let mut counter = 0;
    while let Some(parent) = current_path.parent() {
        counter += 1;
        info!( "[get_project_xml_path] Loop Iteration: {}, Current Path: {}", counter, current_path.display() );
        info!( "[get_project_xml_path] Loop Iteration: {}, Checking Parent Path: {}", counter, parent.display() );
        if counter > 30 {
            error!( "[get_project_xml_path] Search depth exceeded limit from path: {}", item_path.display() );
            return Err(CommandError::from(format!( "Search depth exceeded limit from path: {}", item_path.display() )));
        }

        let parent_name = parent.file_name().and_then(|n| n.to_str());
        info!( "[get_project_xml_path] Loop Iteration: {}, Parent Name Component: {:?}", counter, parent_name );

        if parent_name == Some(HARVEY_FILES_DIR) {
            info!( "[get_project_xml_path] Found '{}' directory: {}", HARVEY_FILES_DIR, parent.display() );
            if let Some(project_base_dir) = parent.parent() {
                info!( "[get_project_xml_path] Project base directory guess: {}", project_base_dir.display() );
                let project_base_name = project_base_dir
                    .file_name()
                    .and_then(|n| n.to_str())
                    .ok_or_else(|| {
                        CommandError::from(format!( "Could not get project base name from {}", project_base_dir.display() ))
                    })?;
                info!( "[get_project_xml_path] Project base name guess: {}", project_base_name );
                let xml_name = format!("{}.harvey.xml", project_base_name);
                let xml_path = project_base_dir.join(xml_name);
                info!( "[get_project_xml_path] Checking for XML file at: {}", xml_path.display() );
                if xml_path.exists() && xml_path.is_file() {
                    info!( "[get_project_xml_path] SUCCESS: Found project XML file: {}", xml_path.display() );
                    return Ok(xml_path);
                } else {
                    error!( "[get_project_xml_path] XML file NOT found or not a file at inferred path: {}", xml_path.display() );
                     return Err(CommandError::from(format!( "Inferred project XML path not found: {}", xml_path.display() )));
                }
            } else {
                 error!( "[get_project_xml_path] Found '{}' but it has no parent directory. Stopping.", HARVEY_FILES_DIR );
                 break; 
            }
        }
        current_path = parent; 
    }

    error!( "[get_project_xml_path] Loop finished WITHOUT success. Could not determine project directory structure from path: {}", item_path.display() );
    Err(CommandError::from(format!( "Could not determine project directory structure from path: {}", item_path.display() )))
}


pub fn get_item_details( item_path: &Path, project_base_dir: &Path,) -> Result<(String, Option<String>, PathBuf), CommandError> {
    let relative_path = item_path.strip_prefix(project_base_dir).map_err(|e| CommandError::from(format!("Failed to strip prefix {} from {}: {}", project_base_dir.display(), item_path.display(), e)))?;
    let components: Vec<&str> = relative_path.components().filter_map(|c| c.as_os_str().to_str()).collect();

    if components.is_empty() {
        return Err(CommandError::from(format!("Item path is the same as project base directory: {}", item_path.display())));
    }

    if components.len() < 2 || components[0] != HARVEY_FILES_DIR {
        return Err(CommandError::from(format!("Item path {:?} not within expected '{}/...' structure.", relative_path, HARVEY_FILES_DIR)));
    }

    let asset_type_dir = components.get(1).copied(); 
    let media_stem = components.get(2).copied(); 
    let sub_folder = components.get(3).copied(); 
    let _filename_comp = components.last().copied();
    let extension = item_path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();

    let media_stem_identifier = if asset_type_dir == Some(MEDIA_DIR) {
        media_stem.map(|s| s.to_string())
    } else {
        None
    };

    let file_type = match (asset_type_dir, sub_folder, extension.as_str()) {
        // --- Rules for files within dedicated stem folders ---
        // For these, `sub_folder` (components[3]) is the filename itself.
        (Some(DOCS_DIR), Some(_), ext) if ["json", "pdf", "md", "txt"].contains(&ext) => "doc".to_string(),
        (Some(IMAGES_DIR), Some(_), ext) if ["jpg", "jpeg", "png", "gif", "bmp", "webp", "tiff"].contains(&ext) => "image".to_string(),
        (Some(TABLES_DIR), Some(_), ext) if ["csv", "xlsx"].contains(&ext) => "table".to_string(),
        (Some(TRANSCRIPTS_DIR), Some(_), "json") => "imported_transcript".to_string(), // Standalone imported transcripts

        // --- Rules for files within specific subdirectories of a MEDIA stem folder ---
        // For these, `sub_folder` (components[3]) is "media" or "transcripts".
        (Some(MEDIA_DIR), Some(MEDIA_SUBDIR), ext) if ["mp3", "wav", "m4a", "ogg", "aac", "flac", "mp4", "mov", "avi", "mkv", "webm"].contains(&ext) => "media".to_string(),
        (Some(MEDIA_DIR), Some(TRANSCRIPTS_SUBDIR), "json") => "transcript".to_string(), // Media-associated transcript

        // --- Legacy/Fallback rules for files directly under asset type dirs (NO dedicated stem folder) ---
        // For these, `sub_folder` (components[3]) would be None.
        (Some(IMAGES_DIR), None, ext) if ["jpg", "jpeg", "png", "gif", "bmp", "webp", "tiff"].contains(&ext) => "image".to_string(),
        (Some(DOCS_DIR), None, "json") => "doc".to_string(),
        (Some(DOCS_DIR), None, "pdf") => "doc".to_string(),
        (Some(DOCS_DIR), None, "md") => "doc".to_string(),
        (Some(DOCS_DIR), None, "txt") => "doc".to_string(),
        (Some(TABLES_DIR), None, "csv") => "table".to_string(),
        (Some(TABLES_DIR), None, "xlsx") => "table".to_string(),
        (Some(TRANSCRIPTS_DIR), None, "json") => "imported_transcript".to_string(), // Legacy standalone

        (Some(MEDIA_DIR), None, _) if components.len() == 3 && item_path.is_dir() => "directory_media_stem".to_string(),
        (Some(MEDIA_DIR), Some(MEDIA_SUBDIR), _) if components.len() == 4 && item_path.is_dir() => "directory".to_string(),
        (Some(MEDIA_DIR), Some(TRANSCRIPTS_SUBDIR), _) if components.len() == 4 && item_path.is_dir() => "directory".to_string(),
        
        (Some(dir_name), None, _) if components.len() == 2 && item_path.is_dir() => {
            match dir_name {
                MEDIA_DIR => "directory_asset_type".to_string(),
                IMAGES_DIR => "directory_asset_type".to_string(),
                TRANSCRIPTS_DIR => "directory_asset_type".to_string(),
                DOCS_DIR => "directory_asset_type".to_string(),
                TABLES_DIR => "directory_asset_type".to_string(),
                _ => "directory".to_string() 
            }
        },
        (_, _, _) if components.len() == 1 && components[0] == HARVEY_FILES_DIR && item_path.is_dir() => "directory_harvey".to_string(),

        _ => {
            warn!("[get_item_details] Unknown file type combination or non-dir not caught by specific file rules: asset_dir={:?}, sub_folder={:?}, ext='{}', is_dir={}, path='{}'", 
                asset_type_dir, sub_folder, extension, item_path.is_dir(), item_path.display());
            "other".to_string()
        }
    };

    Ok((file_type, media_stem_identifier, relative_path.to_path_buf()))
}


pub fn save_project_xml(xml_path: &Path, project_data: &ProjectXml) -> Result<(), CommandError> {
    debug!("[Save XML] Saving to: {:?}", xml_path);
    let xml_string = quick_xml::se::to_string_with_root("project", project_data)
        .map_err(|e| CommandError::from(format!("XML Serialization error: {}", e)))?;
    let final_xml_string = format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n{}", xml_string);
    let file = File::create(xml_path).map_err(|e| CommandError::from(format!("Failed to create/truncate XML file {}: {}", xml_path.display(), e)))?;
    let mut writer = BufWriter::new(file);
    writer.write_all(final_xml_string.as_bytes()).map_err(|e| CommandError::from(format!("Failed to write XML content to {}: {}", xml_path.display(), e)))?;
    writer.flush().map_err(|e| CommandError::from(format!("Failed to flush XML writer for {}: {}", xml_path.display(), e)))?;
    debug!("[Save XML] Success.");
    Ok(())
}


pub fn ensure_base_asset_dirs(project_base_dir: &Path) -> Result<(), CommandError> {
    let base_path = project_base_dir.join(HARVEY_FILES_DIR);
    fs::create_dir_all(base_path.join(MEDIA_DIR))?;
    fs::create_dir_all(base_path.join(IMAGES_DIR))?;
    fs::create_dir_all(base_path.join(TRANSCRIPTS_DIR))?; 
    let docs_path = base_path.join(DOCS_DIR);
    fs::create_dir_all(&docs_path)?;
    fs::create_dir_all(docs_path.join(TEMP_SUBDIR_DOCS))?; 
    fs::create_dir_all(base_path.join(TABLES_DIR))?; 
    debug!("Base asset directories ensured within {}", base_path.display());
    Ok(())
}

pub const MAX_FILENAME_STEM_LENGTH: usize = 60;

pub fn truncate_filename_stem(original_filename: &str, max_stem_len: usize) -> String {
    let path = Path::new(original_filename);
    let original_stem_os = match path.file_stem() {
        Some(s) => s,
        None => {
            // No file stem (e.g., ".bashrc" or just "filename_without_extension")
            // If it starts with a dot and has content after, treat content after dot as stem for truncation purposes if no other extension.
            // Otherwise, the whole name is the stem if no extension.
            if original_filename.starts_with('.') && original_filename.rfind('.').map_or(false, |idx| idx == 0) {
                 // e.g. ".configfile" -> stem is "configfile"
                 return truncate_filename_stem(&original_filename[1..], max_stem_len);
            }
            // If it's like "filename" (no extension) or "nodotextension", the whole thing is the stem.
            // If it's like ".nodotextension" this will be handled by the above.
            // If it's just "." or "..", let it pass, or handle as error if needed, though Path::file_stem handles these reasonably.
            // For simplicity, if file_stem is None, we might be dealing with an edge case like "." or ".." or empty.
            // A robust way is to check if there's any extension-like part.
            if let Some(ext_dot_pos) = original_filename.rfind('.') {
                if ext_dot_pos > 0 && ext_dot_pos < original_filename.len() - 1 { // ensure dot is not first and there's something after
                    let stem_part = &original_filename[..ext_dot_pos];
                    let ext_part = &original_filename[ext_dot_pos..]; // includes the dot
                     if stem_part.len() > max_stem_len {
                        format!("{}{}", &stem_part[..max_stem_len], ext_part)
                    } else {
                        original_filename.to_string()
                    }
                } else { // No clear extension (e.g. "filename" or ".filename_no_further_dot" or "filename.")
                    if original_filename.len() > max_stem_len {
                        original_filename[..max_stem_len].to_string()
                    } else {
                        original_filename.to_string()
                    }
                }
            } else { // No dot at all
                 if original_filename.len() > max_stem_len {
                    original_filename[..max_stem_len].to_string()
                } else {
                    original_filename.to_string()
                }
            }
        }
    };

    let original_stem = original_stem_os.to_string_lossy().into_owned();

    let extension = path.extension().and_then(|s| s.to_str()).map_or(String::new(), |s| format!(".{}", s));

    // Special handling for multi-part extensions like .tar.gz
    // Path::extension only gives the last part (e.g. "gz" from "file.tar.gz")
    // We need to find the first dot that starts the full extension.
    let full_extension = if let Some(first_dot_idx) = original_filename.find('.') {
        if first_dot_idx > 0 { // Ensure it's not a leading dot file like .bashrc
            // Check if the part before the first dot is what Path::file_stem() considers the stem.
            // This is to correctly identify stem for "file.tar.gz" (stem="file", full_ext=".tar.gz")
            // vs "file.name.with.dots.txt" (stem="file.name.with.dots", ext=".txt")
            let potential_stem_from_first_dot = &original_filename[..first_dot_idx];
            if potential_stem_from_first_dot == original_stem || original_stem.starts_with(potential_stem_from_first_dot) {
                 // This indicates 'original_stem' is likely the true stem part we want to truncate.
                 // The 'extension' variable already got the last part (e.g. ".gz").
                 // If original_filename = "file.tar.gz", original_stem = "file.tar", extension = ".gz"
                 // If original_filename = "file.gz", original_stem = "file", extension = ".gz"
                 // What we want is: if original_stem is "file.tar", truncate "file.tar", then add ".gz"
                 // Let's reconstruct the full extension from the original filename if stem is shorter.
                 if original_stem.len() + extension.len() < original_filename.len() {
                     // This implies original_stem from path.file_stem() might not be the part before the *full* extension.
                     // e.g. for "foo.bar.baz.txt", file_stem is "foo.bar.baz", extension is ".txt"
                     // We want to truncate "foo.bar.baz" if it's too long.
                     // The existing `extension` variable is correct for the final extension part.
                 }
            }
             // Fallback: if file_stem gives "archive.tar" and extension gives ".gz",
             // and original is "archive.tar.gz".
             // If original_stem itself contains dots, we are truncating that whole part.
             // e.g. "my.document.name.is.very.long.txt" -> stem is "my.document.name.is.very.long"
        }
        extension // Default to single extension if complex logic above is not perfect.
    } else {
        String::new() // No extension
    };


    if original_stem.len() > max_stem_len {
        let truncated_stem: String = original_stem.chars().take(max_stem_len).collect();
        format!("{}{}", truncated_stem, full_extension)
    } else {
        original_filename.to_string()
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_filename_stem() {
        assert_eq!(truncate_filename_stem("short.txt", 10), "short.txt");
        assert_eq!(truncate_filename_stem("verylongfilenameexample.txt", 10), "verylongfi.txt");
        assert_eq!(truncate_filename_stem("noextension", 5), "noext");
        assert_eq!(truncate_filename_stem("short", 10), "short");
        assert_eq!(truncate_filename_stem(".hiddenfile", 8), ".hiddenfi"); // Path::file_stem gives "hiddenfile"
        assert_eq!(truncate_filename_stem("archive.tar.gz", 10), "archive.ta.gz"); // Path::file_stem is "archive.tar"
        assert_eq!(truncate_filename_stem("archive.tar.gz", 5), "archi.gz");
        assert_eq!(truncate_filename_stem("file.with.many.dots.extension", 15), "file.with.many.extension");
        assert_eq!(truncate_filename_stem("name_without_extension_but_very_long", 10), "name_witho");
        assert_eq!(truncate_filename_stem("another.long.archive.name.tar.zip", 15), "another.long.ar.zip");
        assert_eq!(truncate_filename_stem("single", 3), "sin");
        assert_eq!(truncate_filename_stem(".bashrc", 5), ".bash"); // stem "bashrc"
        assert_eq!(truncate_filename_stem("config.json.backup", 10), "config.jso.backup");
        assert_eq!(truncate_filename_stem("exactlength.info", 11), "exactlength.info");
        assert_eq!(truncate_filename_stem("exactlengthplusone.info", 11), "exactlength.info");
    }
}
