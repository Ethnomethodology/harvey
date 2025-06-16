// src-tauri/src/projectview/shared_types.rs
use serde::{Deserialize, Serialize};
use chrono::Utc; // DateTime removed

// --- Constants ---
pub const HARVEY_FILES_DIR: &str = "harvey_files";
pub const MEDIA_DIR: &str = "Media";
pub const IMAGES_DIR: &str = "Images";
pub const TRANSCRIPTS_DIR: &str = "Transcripts";
pub const DOCS_DIR: &str = "Documents";
pub const TABLES_DIR: &str = "Tables";
pub const MEDIA_SUBDIR: &str = "media";
pub const TRANSCRIPTS_SUBDIR: &str = "transcripts";
pub const TEMP_SUBDIR_DOCS: &str = ".tmp";
pub const METADATA_FILE_SUFFIX: &str = "metadata.json";


// --- Struct Definitions ---

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FileMetadata {
    pub file_name: String,
    pub file_path: String,
    pub last_modified: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_seconds: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame_rate: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bit_rate: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audio_codec: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub video_codec: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>, // Renamed from creation_time
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub original_import_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub speaker_names: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StandardAssetMetadata {
    pub metadata: FileMetadata,
    #[serde(default)]
    pub highlights: Vec<String>,
}


#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct SpeakersXml {
    #[serde(rename = "@count", default)]
    pub count: usize,
    #[serde(default, rename = "name")]
    pub names: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TranscriptSegment {
    pub start_time: f64,
    pub end_time: f64,
    pub speaker: String,
    pub text: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TranscriptEntryXml {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "relativePath")]
    pub relative_path: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DocumentEntryXml {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "relativePath")]
    pub relative_path: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TableEntryXml {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "relativePath")]
    pub relative_path: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ImageEntryXml {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "relativePath")]
    pub relative_path: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ImportedTranscriptEntryXml {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "relativePath")]
    pub relative_path: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DocumentMetadataEntryXml {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "originalDocumentRelativePath")]
    pub original_document_relative_path: String,
    #[serde(rename = "relativePath")]
    pub relative_path: String,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct DocumentFiles {
    #[serde(rename = "documentFile", default)]
    pub files: Vec<DocumentEntryXml>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct TableFiles {
    #[serde(rename = "tableFile", default)]
    pub files: Vec<TableEntryXml>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct ImageFiles {
    #[serde(rename = "imageFile", default)]
    pub files: Vec<ImageEntryXml>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct ImportedTranscriptFiles {
    #[serde(rename = "importedTranscriptFile", default)]
    pub files: Vec<ImportedTranscriptEntryXml>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct DocumentMetadataFiles {
    #[serde(rename = "documentMetadataFile", default)]
    pub files: Vec<DocumentMetadataEntryXml>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MediaFileEntryXml {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "originalPath", skip_serializing_if = "Option::is_none")]
    pub original_path: Option<String>,
    #[serde(rename = "relativePath")]
    pub relative_path: String,
    #[serde(rename = "speakers", skip_serializing_if = "Option::is_none", default)]
    pub speakers: Option<SpeakersXml>,
    #[serde(rename = "transcripts", default, skip_serializing_if = "Vec::is_empty")]
    pub transcripts: Vec<TranscriptEntryXml>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct MediaFiles {
    #[serde(rename = "mediaFile", default)]
    pub files: Vec<MediaFileEntryXml>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(rename = "project")]
pub struct ProjectXml {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(default)] // If project_uuid is missing in XML, it defaults to String::new()
    pub project_uuid: String,
    #[serde(rename = "mediaFiles", default)]
    pub media_files: MediaFiles,
    #[serde(rename = "documentFiles", default)]
    pub document_files: DocumentFiles,
    #[serde(rename = "tableFiles", default)]
    pub table_files: TableFiles,
    #[serde(rename = "imageFiles", default)]
    pub image_files: ImageFiles,
    #[serde(rename = "importedTranscriptFiles", default)]
    pub imported_transcript_files: ImportedTranscriptFiles,
    #[serde(rename = "documentMetadataFiles", default)]
    pub document_metadata_files: DocumentMetadataFiles,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GroupData {
    pub id: String, // UUID
    pub project_id: String, // UUID of the project it belongs to
    pub name: String,
    pub description: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub relative_path: String,
    pub file_type: String,
    pub is_directory: bool,
    #[serde(default)]
    pub parent_relative_path: String,
    #[serde(default)]
    pub depth: usize,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub speakers: Option<SpeakersXml>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub media_xml_identifier: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub associated_transcripts: Vec<TranscriptEntryXml>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<FileEntry>,
}


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProjectViewData {
    pub project_name: String,
    pub project_xml_path: String,
    pub base_directory: String,
    pub project_uuid: String,
    pub files: Vec<FileEntry>,
    pub document_files: Vec<DocumentEntryXml>,
    pub table_files: Vec<TableEntryXml>,
    pub image_files: Vec<ImageEntryXml>,
    pub imported_transcript_files: Vec<ImportedTranscriptEntryXml>,
    pub document_metadata_files: Vec<DocumentMetadataEntryXml>,
}


#[derive(Clone, Serialize, Debug)]
pub struct ProgressPayload {
    pub job_id: String,
    pub percent: f32,
    pub message: String,
}


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TranscriptionResult {
    pub segments: Vec<TranscriptSegment>,
    pub transcript_file_path: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HighlightMetadata {
    pub id: String,
    #[serde(rename = "nodeKey")]
    pub node_key: String,
    pub text: String,
    pub color: String,
    #[serde(default)]
    pub codes: Vec<String>,
    #[serde(default)]
    pub comments: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FileLevelMetadata {
    pub file_name: String,
    pub last_modified: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub summary: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DocumentHighlightData {
    pub metadata: FileMetadata, // Changed type here
    pub highlights: Vec<HighlightMetadata>,
}

impl Default for DocumentHighlightData {
    fn default() -> Self {
        DocumentHighlightData {
            metadata: FileMetadata::default(), // Use FileMetadata's default
            highlights: Vec::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SubtitleFileEntry {
    pub name: String, // e.g., "english.vtt"
    pub path: String, // Full absolute path to the subtitle file
}

// --- Custom Field Definitions ---

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum CustomFieldScope {
    Project,
    AssetType(String), // e.g., "image", "doc", "media"
}

impl CustomFieldScope {
    pub fn to_db_string(&self) -> String {
        match self {
            CustomFieldScope::Project => "project".to_string(),
            CustomFieldScope::AssetType(s) => s.clone(),
        }
    }

    pub fn from_db_string(s: &str) -> Self {
        // This is a simple heuristic. Consider a more robust way if asset types can overlap with "project" string.
        // For now, any string that isn't exactly "project" is treated as an AssetType.
        if s.to_lowercase() == "project" {
            CustomFieldScope::Project
        } else {
            CustomFieldScope::AssetType(s.to_string())
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
// TODO: Add TypeShare derive if this needs to be synced with frontend automatically
// #[derive(Serialize, Deserialize, Debug, Clone, TypeShare)]
pub struct CustomFieldDefinition {
    pub project_id: String, // Added project_id
    pub field_key: String,
    pub field_name: String,
    pub field_type: String, // Example types: "small_text", "long_text", "number", "date", "boolean"
    pub scope: CustomFieldScope,
    pub default_value: Option<String>,
    // Using String for datetime fields for simplicity, assuming they are formatted appropriately by DB
    pub created_at: String,
    pub updated_at: String,
}

// Default implementation for FileMetadata
impl Default for FileMetadata {
    fn default() -> Self {
        FileMetadata {
            file_name: String::new(),
            file_path: String::new(),
            last_modified: Utc::now().to_rfc3339(), // Use chrono::Utc
            title: String::new(),
            description: String::new(),
            summary: String::new(),
            duration_seconds: None,
            width: None,
            height: None,
            frame_rate: None,
            bit_rate: None,
            audio_codec: None,
            video_codec: None,
            created_at: None, // Renamed from creation_time
            original_import_path: None, // New field
            speaker_names: None,        // New field
        }
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize, Debug)]
pub struct AssociatedFile {
    pub name: String,
    pub relative_path: String, // Relative to project base_directory
    pub full_path: String,     // Absolute path
    pub file_type: String,     // e.g., "audio", "video", "document", "image", "table", "imported_transcript", "other"
    pub media_xml_identifier: Option<String>, // For media files, to link to notes, etc.
}

// This struct is primarily for backend use when fetching from DB,
// before enriching with more details for the `AssociatedFile` struct.
#[derive(Debug)]
pub struct FileGroupAssociationFromDb {
    pub file_asset_path: String, // This is the key (relative_path)
    // Potentially add other direct fields from asset_metadata if a JOIN is simple enough
    // pub original_filename: String,
    // pub asset_type_from_db: String,
}


// Default implementation for StandardAssetMetadata
impl Default for StandardAssetMetadata {
    fn default() -> Self {
        StandardAssetMetadata {
            metadata: FileMetadata::default(),
            highlights: Vec::new(),
        }
    }
}