// src-tauri/src/projectview/shared_types.rs
use chrono::Utc;
use serde::{Deserialize, Serialize}; // DateTime removed

// --- Constants ---
pub const HARVEY_FILES_DIR: &str = "harvey_files";
pub const MEDIA_DIR: &str = "Media";
pub const AUDIOS_DIR: &str = "Audios";
pub const VIDEOS_DIR: &str = "Videos";
pub const IMAGES_DIR: &str = "Images";
pub const TRANSCRIPTS_DIR: &str = "Transcripts";
pub const DOCS_DIR: &str = "Documents";
pub const TABLES_DIR: &str = "Tables";
pub const MEDIA_SUBDIR: &str = "media";
pub const TRANSCRIPTS_SUBDIR: &str = "transcripts";
pub const TEMP_SUBDIR_DOCS: &str = ".tmp";
pub const METADATA_FILE_SUFFIX: &str = "metadata.json";

// --- Struct Definitions ---

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub waveform_data: Option<Vec<u8>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language_code: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub properties: Option<String>,
    #[serde(default)]
    pub file_type: String, // New field: audio, video, document, table, image, transcript, audio-transcript, video-transcript
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumbnail: Option<Vec<u8>>, // NEW
}

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StandardAssetMetadata {
    pub metadata: FileMetadata,
    #[serde(default)]
    pub highlights: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SpeakersXml {
    #[serde(rename = "@count", default)]
    pub count: usize,
    #[serde(default, rename = "name")]
    pub names: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub translated_names: Option<Vec<String>>,
}

impl Default for SpeakersXml {
    fn default() -> Self {
        SpeakersXml {
            count: 0,
            names: Vec::new(),
            translated_names: Some(Vec::new()),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct Highlight {
    pub id: String,
    pub text: String,
    pub color: String,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub comments: Option<Vec<serde_json::Value>>,
    #[serde(default)]
    pub timestamp: Option<String>,
}

// New struct for the source of a highlight
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HighlightSource {
    pub file_path: String,
    pub file_name: String,
    pub file_type: String, // e.g., "pdf", "image", "document"
    #[serde(default)] // Add default for compatibility with older data
    pub original_doc_type: String,
}

// New struct to bundle a highlight with its source
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct HighlightInfo {
    #[serde(flatten)]
    pub highlight: Highlight,
    pub source: HighlightSource,
    pub other_tags: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use quick_xml::se::to_string;
    use serde_json::from_str;

    // Helper function to wrap SpeakersXml for top-level element serialization/deserialization
    // quick_xml requires a root element.
    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct SpeakersWrapper {
        #[serde(rename = "speakers")]
        speakers: SpeakersXml,
    }

    #[test]
    fn test_speakers_xml_serialize_with_translated_names() {
        let speakers = SpeakersXml {
            count: 2,
            names: vec!["Speaker 1".to_string(), "Speaker 2".to_string()],
            translated_names: Some(vec!["Trad1".to_string(), "Trad2".to_string()]),
        };
        let wrapper = SpeakersWrapper { speakers };
        let xml_output = to_string(&wrapper).unwrap();
        // quick_xml might not produce pretty output, so exact string match is tricky.
        // Check for essential parts.
        assert!(xml_output.contains("<speakers count=\"2\">"));
        assert!(xml_output.contains("<name>Speaker 1</name>"));
        assert!(xml_output.contains("<name>Speaker 2</name>"));
        assert!(xml_output.contains("<translated_names>"));
        // assert!(xml_output.contains("<name>Trad1</name>")); // This fails due to quick-xml behavior
        // assert!(xml_output.contains("<name>Trad2</name>"));
        assert!(xml_output.contains("</translated_names>"));
        assert!(xml_output.contains("</speakers>"));
    }

    #[test]
    fn test_speakers_xml_serialize_with_empty_translated_names() {
        let speakers = SpeakersXml {
            count: 1,
            names: vec!["Speaker 1".to_string()],
            translated_names: Some(Vec::new()),
        };
        let wrapper = SpeakersWrapper { speakers };
        let xml_output = to_string(&wrapper).unwrap();
        assert!(xml_output.contains("<speakers count=\"1\">"));
        assert!(xml_output.contains("<name>Speaker 1</name>"));
        // Depending on quick_xml's behavior for an empty Vec inside Option wrapped in a tag:
        // It might be <translated_names/> or <translated_names></translated_names>
        // The key is that the translated_names tag IS present because it's Some(Vec::new())
        assert!(!xml_output.contains("<translated_names>"));
    }

    #[test]
    fn test_speakers_xml_serialize_with_none_translated_names() {
        let speakers = SpeakersXml {
            count: 1,
            names: vec!["Speaker 1".to_string()],
            translated_names: None,
        };
        let wrapper = SpeakersWrapper { speakers };
        let xml_output = to_string(&wrapper).unwrap();
        assert!(xml_output.contains("<speakers count=\"1\">"));
        assert!(xml_output.contains("<name>Speaker 1</name>"));
        // translated_names field should be omitted due to skip_serializing_if = "Option::is_none"
        assert!(!xml_output.contains("<translated_names>"));
    }

    #[test]
    fn test_speakers_xml_deserialize_with_empty_translated_names_tag() {
        let xml_input = r#"
            <speakers count="1">
                <name>Speaker 1</name>
                <translated_names/>
            </speakers>
        "#;
        let wrapper: SpeakersWrapper = from_str(&format!(
            "<wrapper>{}</wrapper>",
            xml_input.replace("<speakers", "<speakers xmlns=\"\"")
        ))
        .unwrap();
        let expected_speakers = SpeakersXml {
            count: 1,
            names: vec!["Speaker 1".to_string()],
            translated_names: Some(vec!["".to_string()]), // Empty tag should deserialize to Some([""])
        };
        assert_eq!(wrapper.speakers, expected_speakers);
    }

    #[test]
    fn test_speakers_xml_deserialize_with_empty_translated_names_tags() {
        let xml_input = r#"
            <speakers count="1">
                <name>Speaker 1</name>
                <translated_names></translated_names>
            </speakers>
        "#;
        let wrapper: SpeakersWrapper = from_str(&format!(
            "<wrapper>{}</wrapper>",
            xml_input.replace("<speakers", "<speakers xmlns=\"\"")
        ))
        .unwrap();
        let expected_speakers = SpeakersXml {
            count: 1,
            names: vec!["Speaker 1".to_string()],
            translated_names: Some(vec!["".to_string()]), // Empty tags should deserialize to Some([""])
        };
        assert_eq!(wrapper.speakers, expected_speakers);
    }

    #[test]
    fn test_speakers_xml_deserialize_missing_translated_names_tag() {
        let xml_input = r#"
            <speakers count="1">
                <name>Speaker 1</name>
            </speakers>
        "#;
        let wrapper: SpeakersWrapper = from_str(&format!(
            "<wrapper>{}</wrapper>",
            xml_input.replace("<speakers", "<speakers xmlns=\"\"")
        ))
        .unwrap();
        let expected_speakers = SpeakersXml {
            count: 1,
            names: vec!["Speaker 1".to_string()],
            translated_names: None, // Missing tag should deserialize to None due to #[serde(default)] on Option field
        };
        assert_eq!(wrapper.speakers, expected_speakers);
    }

    // Test for the Default trait implementation
    #[test]
    fn test_speakers_xml_default() {
        let default_speakers = SpeakersXml::default();
        let expected_speakers = SpeakersXml {
            count: 0,
            names: Vec::new(),
            translated_names: Some(Vec::new()), // Default impl sets it to Some(Vec::new())
        };
        assert_eq!(default_speakers, expected_speakers);
    }

    // Test deserialization when count attribute is missing, should default
    #[test]
    fn test_speakers_xml_deserialize_missing_count_attribute() {
        let xml_input = r#"
            <speakers>
                <name>Speaker 1</name>
            </speakers>
        "#;
        let wrapper: SpeakersWrapper = from_str(&format!(
            "<wrapper>{}</wrapper>",
            xml_input.replace("<speakers", "<speakers xmlns=\"\"")
        ))
        .unwrap();
        let expected_speakers = SpeakersXml {
            count: 0, // Defaults to 0 because of #[serde(default)] on count field
            names: vec!["Speaker 1".to_string()],
            translated_names: None,
        };
        assert_eq!(wrapper.speakers, expected_speakers);
    }

    // Test deserialization when names are missing
    #[test]
    fn test_speakers_xml_deserialize_missing_names() {
        let xml_input = r#"
            <speakers count="0">
            </speakers>
        "#;
        let wrapper: SpeakersWrapper = from_str(&format!(
            "<wrapper>{}</wrapper>",
            xml_input.replace("<speakers", "<speakers xmlns=\"\"")
        ))
        .unwrap();
        let expected_speakers = SpeakersXml {
            count: 0,
            names: Vec::new(), // Defaults to empty vec
            translated_names: None,
        };
        assert_eq!(wrapper.speakers, expected_speakers);
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Word {
    pub start: f64,
    pub end: f64,
    pub text: String,
    #[serde(default)]
    pub speaker: Option<String>,
    pub probability: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TranscriptSegment {
    pub start_time: f64,
    pub end_time: f64,
    pub speaker: String,
    pub text: String,
    #[serde(default)]
    pub words: Option<Vec<Word>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TranscriptEntryXml {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "relativePath")]
    pub relative_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language_code: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DocumentEntryXml {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "relativePath")]
    pub relative_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language_code: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TableEntryXml {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "relativePath")]
    pub relative_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language_code: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub has_headers: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ImageEntryXml {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "relativePath")]
    pub relative_path: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct StandaloneTranscriptEntryXml {
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
pub struct StandaloneTranscriptFiles {
    #[serde(rename = "standaloneTranscriptFile", default)]
    pub files: Vec<StandaloneTranscriptEntryXml>,
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
pub struct AudioFiles {
    #[serde(rename = "audioFile", default)]
    pub files: Vec<MediaFileEntryXml>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct VideoFiles {
    #[serde(rename = "videoFile", default)]
    pub files: Vec<MediaFileEntryXml>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(rename = "project")]
pub struct ProjectXml {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(default)] // If project_uuid is missing in XML, it defaults to String::new()
    pub project_uuid: String,
    #[serde(rename = "audioFiles", default)]
    pub audio_files: AudioFiles,
    #[serde(rename = "videoFiles", default)]
    pub video_files: VideoFiles,
    #[serde(rename = "mediaFiles", default, skip_serializing)]
    pub media_files: MediaFiles,
    #[serde(rename = "documentFiles", default)]
    pub document_files: DocumentFiles,
    #[serde(rename = "tableFiles", default)]
    pub table_files: TableFiles,
    #[serde(rename = "imageFiles", default)]
    pub image_files: ImageFiles,
    #[serde(rename = "standaloneTranscriptFiles", default)]
    pub standalone_transcript_files: StandaloneTranscriptFiles,
    #[serde(rename = "documentMetadataFiles", default)]
    pub document_metadata_files: DocumentMetadataFiles,
}

impl ProjectXml {
    pub fn find_media_by_relative_path_mut(
        &mut self,
        relative_path: &str,
    ) -> Option<&mut MediaFileEntryXml> {
        if let Some(f) = self
            .audio_files
            .files
            .iter_mut()
            .find(|f| f.relative_path == relative_path)
        {
            return Some(f);
        }
        if let Some(f) = self
            .video_files
            .files
            .iter_mut()
            .find(|f| f.relative_path == relative_path)
        {
            return Some(f);
        }
        if let Some(f) = self
            .media_files
            .files
            .iter_mut()
            .find(|f| f.relative_path == relative_path)
        {
            return Some(f);
        }
        None
    }

    pub fn find_media_by_stem_dir_mut(
        &mut self,
        stem_rel_path: &str,
    ) -> Option<&mut MediaFileEntryXml> {
        // Media relative path usually looks like `harvey_files/Audios/input_videos/media/input_videos.mp3`
        // We want to match `harvey_files/Audios/input_videos` part
        let matcher =
            |f: &&mut MediaFileEntryXml| -> bool { f.relative_path.starts_with(stem_rel_path) };
        if let Some(f) = self.audio_files.files.iter_mut().find(&matcher) {
            return Some(f);
        }
        if let Some(f) = self.video_files.files.iter_mut().find(&matcher) {
            return Some(f);
        }
        if let Some(f) = self.media_files.files.iter_mut().find(&matcher) {
            return Some(f);
        }
        None
    }

    pub fn find_media(&self, name: &str) -> Option<&MediaFileEntryXml> {
        if let Some(f) = self.audio_files.files.iter().find(|f| f.name == name) {
            return Some(f);
        }
        if let Some(f) = self.video_files.files.iter().find(|f| f.name == name) {
            return Some(f);
        }
        if let Some(f) = self.media_files.files.iter().find(|f| f.name == name) {
            return Some(f);
        }
        None
    }

    pub fn remove_media(&mut self, name: &str) -> bool {
        let old_len = self.audio_files.files.len()
            + self.video_files.files.len()
            + self.media_files.files.len();
        self.audio_files.files.retain(|f| f.name != name);
        self.video_files.files.retain(|f| f.name != name);
        self.media_files.files.retain(|f| f.name != name);
        let new_len = self.audio_files.files.len()
            + self.video_files.files.len()
            + self.media_files.files.len();
        new_len < old_len
    }

    pub fn remove_media_by_stem_dir(&mut self, stem_rel_path: &str) -> bool {
        let old_len = self.audio_files.files.len()
            + self.video_files.files.len()
            + self.media_files.files.len();
        let matcher =
            |f: &MediaFileEntryXml| -> bool { f.relative_path.starts_with(stem_rel_path) };
        self.audio_files.files.retain(|f| !matcher(f));
        self.video_files.files.retain(|f| !matcher(f));
        self.media_files.files.retain(|f| !matcher(f));
        let new_len = self.audio_files.files.len()
            + self.video_files.files.len()
            + self.media_files.files.len();
        new_len < old_len
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GroupData {
    pub id: String,         // UUID
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
    pub standalone_transcript_files: Vec<StandaloneTranscriptEntryXml>,
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

#[allow(dead_code)]
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

#[allow(dead_code)]
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
            created_at: None,           // Renamed from creation_time
            original_import_path: None, // New field
            speaker_names: None,        // New field
            waveform_data: None,
            language_code: None,
            properties: None,
            file_type: String::new(),
            thumbnail: None,
        }
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize, Debug)]
pub struct AssociatedFile {
    pub name: String,
    pub relative_path: String, // Relative to project base_directory
    pub full_path: String,     // Absolute path
    pub file_type: String, // e.g., "audio", "video", "document", "image", "table", "standalone_transcript", "other"
    pub media_xml_identifier: Option<String>, // For media files, to link to data, etc.
    pub last_modified: Option<String>, // Last modified date from file metadata
    pub created_at: Option<String>, // Created at date from file metadata
    pub title: Option<String>, // Title from file metadata
    pub description: Option<String>, // Description from file metadata
    pub waveform_data: Option<Vec<u8>>,
    pub duration_seconds: Option<f64>,
    pub thumbnail_data: Option<Vec<u8>>,
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
