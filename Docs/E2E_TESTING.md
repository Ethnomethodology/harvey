# Harvey E2E Testing (Playwright Virtual Desktop)

This document describes how to run and maintain the End-to-End (E2E) testing suite for the Harvey desktop application.

## Overview

Project Harvey is a Tauri application, which typically requires a native OS environment to run. To enable fast, reliable, and cross-platform E2E testing without a live backend or native window management, we use a **Virtual Desktop Simulation** powered by **Playwright**.

The simulation runs the application's SvelteKit frontend in a standard browser environment while injecting a comprehensive mock of the Tauri backend (the `__TAURI_INTERNALS__` object). This "tricks" the frontend into thinking it is running inside a native Tauri window with full access to file dialogs, system paths, and AI model status.

---

## Prerequisites

Before running the tests, ensure you have installed the project dependencies:

```bash
npm install
```

You may also need to install the Playwright browser binaries if you haven't run Playwright before:

```bash
npx playwright install chromium
```

---

## Running the Tests

### 1. Start the Development Server
The E2E tests target the application running on its local Vite development port (`1420`). In one terminal, start the dev server:

```bash
npm run dev
# Or if you want to use the Tauri CLI:
# npm run tauri dev
```

### 2. Execute the Playwright Suite
In a second terminal, run the specific E2E scenario:

```bash
# Run in headed mode (visible browser window)
npx playwright test e2e-tests/playwright-scenario.spec.js --headed

# Run in headless mode (background)
npx playwright test e2e-tests/playwright-scenario.spec.js
```

---

## Test Results and Screenshots

The test suite automatically captures screenshots of key UI states to verify visual consistency. These are saved directly into the following directory:

`e2e-tests/screenshots/`

Standard screenshots include:
- `1-app-launched.png`: Initial Welcome Screen with the recent projects list.
- `2-project-view-data.png`: The main project view (Data tab).
- `3-transcription-tab.png`: The transcription interface.
- `4-tags-tab.png`: The tag management interface.
- `5-returned-home.png`: The Welcome Screen after "closing" a project.
- `6-final-state.png`: The final state of the application before the test exits.

> [!NOTE]
> The `screenshots/` directory is ignored by Git to avoid repository bloat. You can safely clear this folder at any time.

---

## Maintaining the Simulation (Mocks)

The core logic of the simulation resides in `e2e-tests/playwright-scenario.spec.js`. It uses `page.addInitScript()` to inject a mock Tauri environment before the application loads.

### Adding New Tauri Commands
If you implement a new backend command in Rust (using `#[tauri::command]`) and invoke it in the Svelte frontend, you **must** add a corresponding mock to the `invoke` function in the test script:

```javascript
// e2e-tests/playwright-scenario.spec.js
window.__TAURI_INTERNALS__ = {
  invoke: async (cmd, args) => {
    if (cmd === 'my_new_command') return { success: true, data: "mocked" };
    // ... existing mocks
  }
};
```

### Mocking Tauri V2 Plugins
The simulation explicitly mocks several Tauri v2 plugins:
- **`path`**: Simulates platform-specific path resolution.
- **`dialog`**: Automatically "approves" file save/open requests.
- **`event`**: Simulates the system event bus (`listen`, `emit`).
- **`app`**: Provides version and application name metadata.

---

## Troubleshooting

- **"Loading projects..." hang**: This usually happens if a mocked command is missing or if the `loadProjects` call in `WelcomeScreen.svelte` is blocked by a hanging `await`. Ensure all `invoke` calls in `onMount` hooks are correctly mocked.
- **Port Conflicts**: Ensure no other process is using port `1420`.
- **UI Desync**: If the UI moves too fast for Playwright, the test script includes `await page.waitForTimeout(5000)` calls before screenshots to allow Svelte transitions to settle.
