# Database Schema

**Purpose:** Documents the primary SQLite schema (`harvey.sqlite`) used by the Harvey application. This database acts as the central source of truth for projects, asset metadata, taxonomy (tags/groups), layouts, and configuration.

## Entity-Relationship Diagram

```mermaid
erDiagram
    PROJECTS ||--o{ ASSET_METADATA : contains
    PROJECTS ||--o{ GROUPS : "categorizes into"
    PROJECTS ||--o{ TAG_GROUPS : "defines"
    PROJECTS ||--o{ CUSTOM_FIELD_DEFINITIONS : "defines"
    PROJECTS ||--o{ TABLE_SCHEMAS : "owns"

    GROUPS ||--o{ FILE_GROUPS : "groups"
    ASSET_METADATA ||--o{ FILE_GROUPS : "is grouped by"

    TAG_GROUPS ||--o{ TAGS : "contains"
    PROJECTS ||--o{ TAGS : "owns"

    ASSET_METADATA ||--o{ HIGHLIGHTS : "has annotations"
    HIGHLIGHTS ||--o{ HIGHLIGHT_TAGS : "is tagged with"
    TAGS ||--o{ HIGHLIGHT_TAGS : "tags"

    ASSET_METADATA ||--o{ MEDIA_TRANSCRIPT_DATA : "has transcript config"
    ASSET_METADATA ||--o{ TABLE_LAYOUT_PREFERENCES : "has layout"

    PROJECTS ||--o{ TABLE_CHARTS : "has charts"
    PROJECTS ||--o{ TABLE_VIEWS : "has custom views"
    PROJECTS ||--o{ TABLE_STYLES : "has cell styles"
    PROJECTS ||--o{ PDF_ANNOTATIONS : "has document annotations"

    %% Entities
    PROJECTS {
        TEXT id PK "UUID"
        TEXT name
        TEXT root_path UK
        TEXT xml_path UK
        TIMESTAMP last_opened_ts
    }

    ASSET_METADATA {
        TEXT project_id PK,FK
        TEXT asset_relative_path PK
        TEXT file_name
        TEXT asset_type
        TEXT file_type
        BLOB thumbnail
    }

    GROUPS {
        TEXT id PK
        TEXT project_id FK
        TEXT name
    }

    FILE_GROUPS {
        INTEGER id PK
        TEXT file_asset_path FK
        TEXT group_id FK
        TEXT project_id FK
    }

    TAG_GROUPS {
        TEXT id PK
        TEXT project_id FK
        TEXT name
    }

    TAGS {
        INTEGER id PK
        TEXT project_id FK
        TEXT name
        TEXT color
        INTEGER tag_group_id FK
    }

    HIGHLIGHTS {
        INTEGER id PK
        TEXT asset_id FK
        TEXT project_id FK
        INTEGER start_offset
        INTEGER end_offset
        TEXT text
        TEXT annotation_id
    }

    HIGHLIGHT_TAGS {
        INTEGER highlight_id PK,FK
        INTEGER tag_id PK,FK
        TEXT project_id FK
    }

    GLOBAL_SETTINGS {
        TEXT key PK
        TEXT value
    }

    DOWNLOADED_MODELS {
        TEXT name PK
        TEXT family
        TEXT download_location
    }
```

## Table Breakdown

### Core Hierarchy
*   **`projects`**: The root registry of all workspaces. Contains the canonical UUIDs (`id`) that link every other piece of data in the DB to a specific project scope. Tracks `last_opened_ts` to power the "Recent Projects" view.
*   **`asset_metadata`**: The central repository for all files tracked by a project (videos, audio, documents, tables, transcripts). Uses a composite primary key (`project_id`, `asset_relative_path`). It stores highly variable media metadata (codecs, framerates), custom JSON fields, cached waveform blob data, and thumbnails.
*   **`custom_field_definitions`**: Defines the schema for the dynamic key-value properties users can assign to files, enforcing types (e.g., text, number) and scopes across a project.

### Organization & Taxonomy
*   **`groups`**: User-defined folders or collections within a project.
*   **`file_groups`**: A relational mapping table placing specific `asset_metadata` rows into specific `groups`.
*   **`tag_groups`**: High-level categories (e.g., "Sentiment", "Speakers") to organize individual tags.
*   **`tags`**: Individual semantic labels (e.g., "Positive", "John Doe") with specific colors, globally available to a project.

### Annotations & Highlights
*   **`highlights`**: Represents a specific spatial or temporal selection within an asset (e.g., a text span in a transcript, a region in an image).
*   **`highlight_tags`**: A many-to-many mapping connecting `highlights` to `tags`.
*   **`pdf_annotations`**: A legacy/specialized table storing serialized JSON blobs of complex vector shapes or Lexical nodes specific to Documents, Images, and PDFs.

### Asset-Specific Configurations
*   **`media_transcript_data`**: Stores transcription-specific overrides (like language codes, initial prompts, or hotwords) for audio/video assets.
*   **`table_schemas`**: Serialized JSON defining the column types (e.g., Currency, Progress, DateTime) for CSV/XLSX assets.
*   **`table_styles`**: Saved cell/row background colors and text formatting for grid views.
*   **`table_charts`** & **`table_views`**: Serialized JSON configurations saving user-built graphs or Pivot Table layouts.
*   **`table_layout_preferences`**: State memory (like column widths or hidden columns) for grid views.

### Global Application State
*   **`global_settings`**: A simple key-value store for application-wide preferences (e.g., theme, download paths) that persist regardless of which project is open.
*   **`downloaded_models`**: A registry of local machine learning models (Faster-Whisper, Pyannote) available to the application.

## Conventions
*   **Foreign Keys (`FK`)**: Heavily utilizes `ON DELETE CASCADE`. If a `project` is deleted, its `asset_metadata`, `groups`, and `tags` are automatically wiped. If an `asset_metadata` row is deleted, its `file_groups` and `highlights` are wiped.
*   **Timestamps**: Almost all tables utilize `created_at` and `updated_at` columns, relying on SQLite `AFTER UPDATE` triggers defined in `db_handler.rs` to automatically keep `updated_at` current.
*   **Paths**: All paths stored in tables (except for `projects.root_path`) are stored as **relative paths** from the project root. This ensures that entire Harvey project folders can be moved on the file system without breaking database links.