# Harvey Backend Refactoring & Database Schema

## Backend Refactoring Summary (October 2023 - January 2024)

The backend of the Harvey application underwent a significant refactoring process recently. The primary goals of this refactoring were to streamline data management, improve consistency, and prepare for future feature enhancements. Key changes involved consolidating metadata storage, standardizing project identification, and refining command structures for frontend interaction.

A major part of this effort was the introduction of a centralized SQLite database (`harvey.sqlite`) to manage metadata that was previously stored in disparate JSON files (e.g., `.harvey_metadata.json` for documents, `.metadata.json` for media, tables, and images). This includes asset-specific details, custom fields, and project information. The `projects` table now serves as a central registry for all projects, using a unique UUID (`id`) for each. This `project_id` is then used as a foreign key in other tables like `asset_metadata` to link assets to their respective projects, ensuring data integrity and simplifying queries.

During this transition, several commands were updated to interact with the new database structure. For instance, functions like `import_media`, `import_document_files`, `import_table_file`, `import_image_file`, and `import_word_transcript` now fetch the `project_uuid` from the project's XML file and pass it as `project_id` when saving asset metadata via `db_handler::save_asset_metadata`. Similarly, functions in `metadata_commands.rs` (like `update_asset_metadata_command`) and `welcome/commands.rs` (for project creation) were modified to ensure project information and UUIDs are correctly registered and utilized in the database. Error handling for XML parsing was also standardized to use `CommandError::XmlDeserialization`.

Currently, there's an identified issue on the frontend where the `update_asset_metadata_command` is called without providing the necessary `project_xml_path_str` argument. This prevents the backend from extracting the `project_id` (UUID) from the project's XML file, which is crucial for saving the metadata to the correct project scope in the `asset_metadata` table. The solution involves ensuring the frontend client correctly retrieves and passes the `project_xml_path_str` (the path to the `.harvey.xml` file for the currently open project) when invoking `update_asset_metadata_command`. Once this path is provided, the backend can read the XML, extract the `project_uuid`, and correctly associate the asset metadata with its project in the database.

## Database Schema Documentation

This document outlines the schema for the SQLite database used by the Harvey application.

### Global Configuration and Conventions

*   **Database File**: The database is stored in a single file named `harvey.sqlite` located in the application's configuration directory.
*   **Timestamps**: Standard columns `created_at` and `updated_at` are used across multiple tables.
    *   `created_at`: Timestamp of when a record was first inserted. Defaults to `CURRENT_TIMESTAMP`.
    *   `updated_at`: Timestamp of when a record was last modified. Defaults to `CURRENT_TIMESTAMP` and is automatically updated by a trigger on relevant tables.
*   **Foreign Keys**: Foreign key constraints with `ON DELETE CASCADE` are used where appropriate to maintain data integrity. If a referenced project or asset is deleted, related data in other tables will also be removed.
*   **Project ID**: The `projects.id` (a UUID string) is a crucial foreign key in many tables, linking data to a specific project.
*   **Asset Paths**: Asset paths stored (e.g., `asset_relative_path` in `asset_metadata`) are typically relative to the project's root directory.

---

### Table of Contents

-   [projects](#projects)
-   [asset_metadata](#asset_metadata)
-   [pdf_annotations](#pdf_annotations)
-   [custom_field_definitions](#custom_field_definitions)
-   [table_layout_preferences](#table_layout_preferences)

---

## `projects`

Stores information about each project created in the application.

| Column       | Type      | Constraints                   | Description                                                                 |
|--------------|-----------|-------------------------------|-----------------------------------------------------------------------------|
| `id`         | `TEXT`    | `PRIMARY KEY`                 | The unique identifier (UUID v4) for the project.                            |
| `name`       | `TEXT`    | `NOT NULL`                    | The user-defined name of the project.                                       |
| `root_path`  | `TEXT`    | `NOT NULL`, `UNIQUE`          | The absolute canonical path to the project's root directory on the filesystem. |
| `xml_path`   | `TEXT`    | `NOT NULL`, `UNIQUE`          | The absolute canonical path to the project's main XML file (e.g., `ProjectName.harvey.xml`). |
| `created_at` | `TIMESTAMP`| `DEFAULT CURRENT_TIMESTAMP`   | Timestamp of when the project record was created.                           |
| `updated_at` | `TIMESTAMP`| `DEFAULT CURRENT_TIMESTAMP`   | Timestamp of when the project record was last updated. Auto-updates on change. |

**Triggers:**
*   An `AFTER UPDATE` trigger automatically sets `updated_at` to `CURRENT_TIMESTAMP` when a row in this table is updated.

---

## `asset_metadata`

Stores metadata for various assets (media files, documents, tables, images, imported transcripts) within projects.

| Column                | Type      | Constraints                                      | Description                                                                    |
|-----------------------|-----------|--------------------------------------------------|--------------------------------------------------------------------------------|
| `project_id`          | `TEXT`    | `REFERENCES projects(id) ON DELETE CASCADE`      | Foreign key linking to the `projects` table. Ensures assets are tied to a project. |
| `asset_relative_path` | `TEXT`    | `PRIMARY KEY`                                    | The path of the asset relative to the project's base directory. Used as a unique key for the asset. |
| `file_name`           | `TEXT`    | `NOT NULL`                                       | The name of the asset file (e.g., `video.mp4`, `document.json`).                 |
| `file_path`           | `TEXT`    | `NOT NULL`                                       | The absolute path to the asset file on the filesystem.                           |
| `last_modified`       | `TEXT`    | `NOT NULL`                                       | Timestamp (ISO 8601 string) of when the asset file was last modified or when the metadata record was significantly updated. |
| `title`               | `TEXT`    |                                                  | User-defined title for the asset.                                              |
| `description`         | `TEXT`    |                                                  | User-defined description for the asset.                                        |
| `summary`             | `TEXT`    |                                                  | User-defined summary for the asset.                                            |
| `duration_seconds`    | `REAL`    |                                                  | For media assets, the duration in seconds.                                     |
| `width`               | `INTEGER` |                                                  | For visual media (video, images), the width in pixels.                         |
| `height`              | `INTEGER` |                                                  | For visual media (video, images), the height in pixels.                        |
| `frame_rate`          | `REAL`    |                                                  | For video assets, the frame rate.                                              |
| `bit_rate`            | `INTEGER` |                                                  | For media assets, the bit rate in bits per second.                             |
| `audio_codec`         | `TEXT`    |                                                  | For media assets, the audio codec.                                             |
| `video_codec`         | `TEXT`    |                                                  | For video assets, the video codec.                                             |
| `creation_time`       | `TEXT`    |                                                  | Timestamp (ISO 8601 string from media metadata) of when the asset was created.   |
| `asset_type`          | `TEXT`    | `NOT NULL`                                       | Type of the asset (e.g., 'video', 'audio', 'document', 'table', 'image', 'transcript'). |
| `custom_fields_json`  | `TEXT`    |                                                  | A JSON string storing custom field values associated with this asset.          |
| `created_at`          | `TIMESTAMP`| `DEFAULT CURRENT_TIMESTAMP`                      | Timestamp of when the asset metadata record was created.                         |
| `updated_at`          | `TIMESTAMP`| `DEFAULT CURRENT_TIMESTAMP`                      | Timestamp of when the asset metadata record was last updated. Auto-updates on change. |

**Triggers:**
*   An `AFTER UPDATE` trigger automatically sets `updated_at` to `CURRENT_TIMESTAMP` when a row in this table is updated.

**Relationships:**
*   `project_id` links to `projects.id`.

---

## `pdf_annotations`

Stores annotations made on PDF documents. This table might also be used for image annotations if the `document_type` is set to 'image' and `pdf_document_path` stores the image's relative asset path.

| Column              | Type      | Constraints                        | Description                                                                    |
|---------------------|-----------|------------------------------------|--------------------------------------------------------------------------------|
| `id`                | `INTEGER` | `PRIMARY KEY AUTOINCREMENT`        | Auto-incrementing primary key for the annotation record.                       |
| `pdf_document_path` | `TEXT`    | `NOT NULL`, `UNIQUE`               | The path to the PDF document or image asset, conceptually relative to the project or a known base. This path is the key for annotations. It should correspond to an `asset_relative_path` in `asset_metadata`. |
| `annotations_json`  | `TEXT`    | `NOT NULL`                         | A JSON string containing the array of annotations.                 |
| `document_type`     | `TEXT`    | `NOT NULL DEFAULT 'pdf'`           | Type of the document being annotated (e.g., 'pdf', 'image').                   |
| `created_at`        | `TIMESTAMP`| `DEFAULT CURRENT_TIMESTAMP`        | Timestamp of when the annotation record was created.                           |
| `updated_at`        | `TIMESTAMP`| `DEFAULT CURRENT_TIMESTAMP`        | Timestamp of when the annotation record was last updated. Auto-updates on change.|

**Triggers:**
*   An `AFTER UPDATE` trigger automatically sets `updated_at` to `CURRENT_TIMESTAMP` when a row in this table is updated.

---

## `custom_field_definitions`

Stores definitions for custom fields that can be applied to assets or projects.

| Column        | Type      | Constraints                                      | Description                                                                    |
|---------------|-----------|--------------------------------------------------|--------------------------------------------------------------------------------|
| `project_id`  | `TEXT`    | `NOT NULL`, `PRIMARY KEY` (composite)            | Foreign key linking to the `projects` table. Part of the composite primary key.  |
| `field_key`   | `TEXT`    | `NOT NULL`, `PRIMARY KEY` (composite)            | Unique key for the custom field within a project. Part of the composite primary key. |
| `field_name`  | `TEXT`    | `NOT NULL`                                       | Display name for the custom field.                                             |
| `field_type`  | `TEXT`    | `NOT NULL`                                       | Data type of the custom field (e.g., 'text', 'number', 'date').                |
| `scope`       | `TEXT`    | `NOT NULL`                                       | Scope of the custom field (e.g., 'project', or an asset type like 'image', 'document'). |
| `default_value`| `TEXT`   |                                                  | Default value for the custom field.                                            |
| `created_at`  | `TIMESTAMP`| `NOT NULL DEFAULT CURRENT_TIMESTAMP`             | Timestamp of when the custom field definition was created.                     |
| `updated_at`  | `TIMESTAMP`| `NOT NULL DEFAULT CURRENT_TIMESTAMP`             | Timestamp of when the custom field definition was last updated. Auto-updates on change. |

**Primary Key:** (`project_id`, `field_key`)

**Triggers:**
*   An `AFTER UPDATE` trigger automatically sets `updated_at` to `CURRENT_TIMESTAMP` when a row in this table is updated.

**Relationships:**
*   `project_id` links to `projects.id`.

---

## `table_layout_preferences`

Stores user layout preferences for specific table assets.

| Column                      | Type      | Constraints                                      | Description                                                                    |
|-----------------------------|-----------|--------------------------------------------------|--------------------------------------------------------------------------------|
| `table_asset_relative_path` | `TEXT`    | `PRIMARY KEY`, `REFERENCES asset_metadata(asset_relative_path) ON DELETE CASCADE` | The relative path of the table asset this layout preference applies to. Foreign key to `asset_metadata`. |
| `layout_json`               | `TEXT`    | `NOT NULL`                                       | A JSON string defining the layout preferences for the table.                   |
| `updated_at`                | `TIMESTAMP`| `NOT NULL DEFAULT CURRENT_TIMESTAMP`             | Timestamp of when the layout preference was last updated. Auto-updates on change. |

**Triggers:**
*   An `AFTER UPDATE` trigger automatically sets `updated_at` to `CURRENT_TIMESTAMP` when a row in this table is updated.

**Relationships:**
*   `table_asset_relative_path` links to `asset_metadata.asset_relative_path`.
