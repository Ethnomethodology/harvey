# Project View Components

**Purpose:** This directory houses the primary application views and orchestrators that the user interacts with once a project has been successfully opened or created.

## Overview

The `projectview` directory acts as the main shell for all tabbed experiences within the application (Data, Transcription, Tags, etc.). Rather than holding raw components, it is strictly organized into domain-specific subdirectories.

Each subdirectory contains its own `README.md` documenting its specific architecture, component flow, Tauri IPC integration, and expected behaviors.

## Subdirectories

- **`data/`**
  - Contains the UI for the "Data" tab, orchestrating file browsing, document/table previews, and dynamic context panels (like Metadata and Attachments).
  - See [`data/README.md`](./data/README.md) for full details.

- **`transcription/`**
  - Contains the UI for the "Transcription" tab, including synchronized media players, editable segmented transcripts, horizontal/vertical waveforms, and rich-text continuous previews.
  - See [`transcription/README.md`](./transcription/README.md) for full details.

- **`tags/`**
  - Contains the UI for the global taxonomy explorer, including the threaded comment side-panels and global tag filtering.
  - See [`tags/README.md`](./tags/README.md) for full details.

- **`shared/`**
  - Contains highly reusable components built for cross-tab usage, such as the `MediaPlayer.svelte`, Svelte Flowbite extensions, context menus, and global toast notifications.
  - See [`shared/README.md`](./shared/README.md) for full details.

- **`modals/`**
  - Houses all floating dialog components specific to active projects, such as export configurations, data entry dialogs (like `EditEntryModal.svelte`), and confirmation dialogs. Note that global application modals (like settings) live in the higher-level `src/lib/components/modals` directory.
  - See [`modals/README.md`](./modals/README.md) for full details.

- **`lexical/`**
  - Contains the custom, highly integrated Lexical rich-text editor wrapper, custom plugins (Nodes), and floating toolbars used for highlighting and tagging text natively.
  - See [`lexical/README.md`](./lexical/README.md) for full details.

- **`tables/`**
  - Contains specific context menus and floating toolbars engineered explicitly for the Lexical table plugin.
  - See [`tables/README.md`](./tables/README.md) for full details.

## Orchestration Flow

Typically, a top-level orchestrator like `ProjectView.svelte` (found directly in `src/routes/` or a higher directory) acts as the router between these sub-domains. For example, selecting the "Data" tab mounts the orchestrator from `data/DataView.svelte`, while selecting "Transcription" mounts `transcription/TranscriptionView.svelte`. State sync between these distinct tabs is handled entirely by Svelte Stores inside `src/lib/stores`.
