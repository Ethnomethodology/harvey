# AI Agent Instructions

These instructions are binding for any AI agent (e.g., Jules) interacting with the Harvey codebase.

## 1. Documentation is the Source of Truth
The Harvey application is complex, utilizing SvelteKit, Tauri IPC, SQLite, and Python subprocesses. It relies on a strictly enforced **"Visual First" documentation standard**.

Before attempting to implement a new feature, modify a component, or debug an issue, you **MUST**:
1. Identify the directory housing the target code (e.g., `src/lib/components/projectview/data/`).
2. Read the colocated `README.md` file within that directory to understand the component architecture, required Props, Svelte Stores, and specific Tauri IPC Commands.
3. Review `Docs/DATABASE_SCHEMA.md` if your task involves modifying database schemas or Rust `db_handler.rs` queries.

Do not attempt to infer complex component lifecycles or store dependencies solely by reading the source code without first consulting the relevant `README.md`.

## 2. Mandatory Documentation Updates
When you modify code that alters the component architecture, adds new Props, introduces new Svelte Stores, invokes a new Tauri backend command, or changes a database table, you **MUST**:
1. Locate the corresponding `README.md` file (or `Docs/DATABASE_SCHEMA.md`).
2. Update the Mermaid diagrams (e.g., `block-beta`, `flowchart TD`, `erDiagram`) to reflect your changes.
3. Update the textual breakdown (Props, State, IPC Commands).
4. Ensure your updates strictly conform to the templates defined in `CONTRIBUTING.md`.

You are not finished with a feature or bug fix until the documentation is updated to match the new code state.

## 3. Styling Constraints
* Use **Tailwind CSS** utility classes directly in the markup.
* Use **Flowbite-Svelte** components for interactive UI elements (Modals, Inputs, Buttons) where possible.
* Support Dark Mode by explicitly defining `dark:` variant classes alongside light mode classes (e.g., `bg-white dark:bg-gray-900 text-gray-900 dark:text-gray-100`).
* Refer to `Docs/STYLE_GUIDE.md` for the core color palette mapping.