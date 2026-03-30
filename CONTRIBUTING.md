# Documentation Standards and Contribution Guidelines

This document outlines the standard templates and practices for documenting the Harvey codebase. We enforce a "Visual First" documentation standard, meaning that diagrams (via Mermaid) should be used heavily to communicate architecture, data flow, and UI layouts.

All major architectural components, services, and stores must be documented using colocated `README.md` files. When adding or refactoring features, it is your responsibility to update the corresponding documentation to keep it accurate.

---

## 1. Frontend UI Components (`src/lib/components/`)

Use this template for documenting folders containing Svelte UI components. Treat the primary orchestrating component of the folder as the main subject.

```markdown
# [Insert Main Component Name]

**Purpose:** [Write a concise, one-sentence summary of what this UI piece does within the application].

## Visual Wireframe
[Create a visual representation of the UI layout using Mermaid's `block-beta` syntax.]
\`\`\`mermaid
block-beta
  columns 3
  LeftPanel["Left Panel"]:1 MainContent["Main Content Area"]:2
\`\`\`

## Component Architecture
[Create a Mermaid `flowchart TD` diagram showing this component, its internal child components, and how data/props flow between them.]
\`\`\`mermaid
flowchart TD
    ParentComponent --> ChildComponentA
    ParentComponent --> ChildComponentB
\`\`\`

## Props / Inputs
* **`[Prop Name]`** (`[Type]`): [Brief description of what it is and where it comes from].

## State & Context (Svelte Stores)
* **Local State:** [Describe any local state managed here, e.g., standard let variables, derived state. If none, say "None"].
* **Global Stores:** [List the exact Svelte stores from `src/lib/stores/` this component subscribes to or updates].

## Backend & Database Interop (Tauri IPC)
* **Tauri Commands Triggered:** [List any Rust commands called directly via `invoke()` or through service files].
* **Data Flow:** [Briefly explain how the database payload is handled once received].

## Child Components
* **`[ComponentName]`** (`[FilePath]`): [Briefly note its purpose].

## Expected Behaviors & Edge Cases
* **[Action/Trigger]:** [Explain what happens, e.g., "When the refresh button is clicked..."].
* **[Edge Case]:** [Explain how the UI handles empty states, loading states, or null data].
```

---

## 2. Backend Rust Modules (`src-tauri/src/`)

Use this template for documenting Rust backend modules, typically placed at the root of a module folder (e.g., `src-tauri/src/projectview/README.md`).

```markdown
# Module: [Insert Module Name, e.g., ProjectView]

**Purpose:** [One-sentence summary of what this backend module handles].

## Architecture & Data Flow
*Use a Mermaid flowchart to map how frontend calls route through the commands down to the handlers and external systems.*
\`\`\`mermaid
flowchart LR
    Frontend([Svelte Frontend]) -. "invoke('command_name')" .-> Commands[module_commands.rs]
    Commands --> Handlers[module_handler.rs]
    Handlers --> DB[(SQLite Database)]
    Handlers --> FS[File System]
\`\`\`

## Tauri IPC Commands (The API Surface)
*List the `#[tauri::command]` functions defined in this module.*
* **`command_name(args)`** -> `Result<Type, Error>`: [Brief description of what it does and which handler it calls].

## Internal Handlers
*Briefly describe the purpose of the internal `_handler.rs` files.*
* **`module_handler.rs`**: [e.g., Executes raw SQL queries].

## Managed State & Concurrency
*List any Tauri managed state (`tauri::State`) or `Mutex`/`RwLock` structures.*
* **`[StateStruct]`**: [Why it is needed and what it locks/manages].

## Expected Errors
*How does this module fail, and what does it return to the frontend?*
* **[Error Type]**: [e.g., Returns a serialized string error if the file path is not found].
```

---

## 3. Frontend Stores (`src/lib/stores/`)

Use this template for documenting global state management.

```markdown
# Store: [Insert Store Name, e.g., ProjectStore]

**Purpose:** [One-sentence summary of the global state managed here].

## Store Dependency Graph
*Visually map if this store subscribes to or derives from any other stores.*
\`\`\`mermaid
flowchart LR
    StoreA --> DerivedStoreB
    DerivedStoreB --> FinalState
\`\`\`

## State Shape
*Describe the primary data structure of the store.*
* **`[Key]`** (`[Type]`): [Brief description of what it holds].

## Derived State
*List any `derived` stores that compute values based on this store.*
* **`[Derived Store Name]`**: [Explanation of what it computes].

## Actions / Mutations
*List the exported functions that modify the store.*
* **`[FunctionName(args)]`**: [What it does and what state it mutates].

## Subscriptions / Effects
*Describe any side effects triggered by subscribing to this store or other stores it listens to.*
```

---

## 4. Frontend Services (`src/lib/services/`)

Use this template for documenting business logic and API abstractions.

```markdown
# Service: [Insert Service Name, e.g., ProjectService]

**Purpose:** [One-sentence summary of the business logic or API abstraction provided].

## Interop Sequence
*Visually map the lifecycle from Svelte to Rust and back.*
\`\`\`mermaid
sequenceDiagram
    participant Component
    participant Service
    participant TauriIPC
    participant Store

    Component->>Service: Call Function
    Service->>TauriIPC: invoke('command')
    TauriIPC-->>Service: Return Data
    Service->>Store: Update State
\`\`\`

## Exported Functions
*List the main functions exported by this service.*
* **`[FunctionName(args)]`** -> `Promise<Type>`: [What it does].

## Tauri IPC / External API Calls
*List the backend commands invoked by this service.*
* **`invoke('command_name')`**: [Why it is called and what it returns].

## Data Transformation
*Briefly explain if this service normalizes or transforms data before returning it to the components or stores.*
```

---

## 5. Routes (`src/routes/`)

Use this template for documenting page routing and layout composition.

```markdown
# Route: [Insert Route Path, e.g., /project/[id]]

**Purpose:** [One-sentence summary of the page's purpose].

## Routing & Layout
*Visually map how the URLs map to layouts and pages.*
\`\`\`mermaid
flowchart TD
    RootLayout["Root Layout (+layout.svelte)"] --> Route["/project/[id] (+page.svelte)"]
    Route --> PageComponent["Main Page Component"]
\`\`\`

## Data Loading
*Describe what data is fetched in `+page.js` or `+page.server.js` before rendering.*
* **`load({ params })`**: [What data is fetched and passed as props].

## Top-Level Components
*List the primary components rendered by this route.*
* **`[ComponentName]`**: [Brief description].
```

---

## 6. Web Workers (`src/lib/workers/`)

Use this template for documenting heavy computation offloaded to background threads.

```markdown
# Worker: [Insert Worker Name, e.g., PdfAnnotationMatcher]

**Purpose:** [One-sentence summary of the heavy computation offloaded to this worker].

## Input Messages (Main Thread -> Worker)
*Describe the payload structure the worker expects.*
* **`[Event/Command Type]`**: `{ [key: type] }` [What data is sent to the worker].

## Output Messages (Worker -> Main Thread)
*Describe the payload structure the worker sends back.*
* **`[Result Type]`**: `{ [key: type] }` [What data is returned upon completion or error].

## Processing Logic
*Briefly explain the algorithm or processing steps performed by the worker.*
```

---

## 7. Utilities & Constants (`src/lib/utils/`, `src/lib/constants/`)

Use this simpler template for documenting helpers and static values.

```markdown
# Module: [Insert File Name, e.g., DateFormatUtils or ThemeConstants]

**Purpose:** [One-sentence summary].

## Exported Utilities / Constants
*List the main exports.*
* **`[FunctionName / CONSTANT_NAME]`**: [What it does or what value it holds].

## Usage Example
*Provide a brief code snippet showing how to use the primary export.*
```
