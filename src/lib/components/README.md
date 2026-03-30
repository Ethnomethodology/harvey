# Components Library

**Purpose:** This directory (`src/lib/components`) houses all Svelte presentation and container components used across the entire desktop application.

## Directory Structure

The components library is split into two primary domains, reflecting the distinct application states: the launch screen (where no project is open) and the active workspace (where a project is loaded). It also contains shared, global resources.

### 1. `welcome/`
Contains the **Welcome Screen** components. This is the initial interface users interact with to create new projects, open existing ones, configure the application, or access the Setup Wizard.
* **Key Components:** `WelcomeScreen.svelte`, `ProjectList.svelte`, `SetupWizardModal.svelte`.
* See [`welcome/README.md`](./welcome/README.md) for full details.

### 2. `projectview/`
Contains the **Active Workspace** components. Once a project is loaded, these components orchestrate the primary tabs (Data, Transcription, Tags) and all associated media playback, rich-text editing, and file management UIs.
* **Subdirectories:** `data/`, `transcription/`, `tags/`, `lexical/`, `shared/`, `modals/`, `tables/`.
* See [`projectview/README.md`](./projectview/README.md) for full details and links to domain-specific documentation.

### 3. `modals/`
Houses global, application-wide modal dialogs that are not inherently tied to a specific sub-view or project state. These can be invoked from anywhere.
* **Key Components:** `ConfigurationModal.svelte`, `HelpModal.svelte`, `InstallLogModal.svelte`.
* See [`modals/README.md`](./modals/README.md) for full details.

### 4. `shared/`
Contains highly reusable, top-level functional components—primarily focusing on the application configuration and settings panels. These are utilized by both the Welcome Screen and inside active projects.
* **Key Components:** `ConfigurationView.svelte`, `LibrariesPanel.svelte`, `TranscriptionConfiguration.svelte`.
* See [`shared/README.md`](./shared/README.md) for full details.

## Documentation Standards
All subdirectories within `src/lib/components` must adhere to the "Visual First" documentation standard defined in the root [`CONTRIBUTING.md`](../../../CONTRIBUTING.md). This ensures every UI module includes:
1. A clear statement of purpose.
2. Mermaid-based visual wireframes (`block-beta`).
3. Mermaid-based component architecture diagrams (`flowchart TD`).
4. Detailed breakdown of Svelte Stores and Tauri IPC integration.
