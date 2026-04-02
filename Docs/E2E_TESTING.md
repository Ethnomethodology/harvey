# E2E Testing Guide for Harvey

This application uses **WebDriverIO (WDIO)** in combination with `tauri-plugin-webdriver-automation` to run true end-to-end tests against the compiled native binary (`src-tauri/target/debug/harvey`).

## 1. Prerequisites

To run these tests locally on any platform, you must have the `tauri-wd` (Tauri WebDriver) CLI tool installed.

```bash
cargo install tauri-webdriver-automation
```

Ensure your `~/.cargo/bin` is in your `PATH` so `tauri-wd` can be executed from your terminal.

---

## 2. Running Tests Locally

> [!IMPORTANT]
> **Always rebuild the application in debug mode before running tests.**
> Unlike `tauri dev` (which serves a live Vite dev server), the e2e tests launch the compiled
> native binary directly. That binary has the `build/` directory **embedded at compile time**.
> If you skip this step, the test screenshots will render with stale or missing styles.

### Step 1 — Build the Debug Binary
```bash
npm run tauri build -- --debug
```
This automatically builds the SvelteKit frontend and embeds `build/` into the Tauri debug binary.

### Step 3 — Start the WebDriver server (macOS & Linux)
```bash
tauri-wd --port 4444 &
```

### Step 4 — Run the test suite
```bash
nx wdio run wdio.conf.mjs
```

Or as a single pipeline:
```bash
npm run tauri build -- --debug && npx wdio run wdio.conf.mjs
```

### Windows
1. Follow Step 1 above, then open PowerShell and start the WebDriver server:
   ```powershell
   Start-Process tauri-wd -ArgumentList "--port 4444" -NoNewWindow
   ```
2. Run the test suite:
   ```powershell
   npx wdio run wdio.conf.mjs
   ```

> **Note:** The `wdio.conf.mjs` is configured to automatically detect your operating system and append `.exe` to the binary path on Windows.

---

## 3. Running Tests in a Cloud Agent or Virtual Environment (Linux)

Cloud agents and CI/CD pipelines (like GitHub Actions) often run in "headless" Linux containers without graphical displays. Because Tauri is a native desktop application, it will crash (`Failed to initialize GTK`) if it cannot find an attached display.

To run the tests successfully in a Linux virtual environment, you must use **Xvfb** (X Virtual Framebuffer) to trick the application into thinking a monitor is attached.

### Setup (Debian/Ubuntu)
```bash
sudo apt-get update
sudo apt-get install -y xvfb
```

### Execution
Wrap both the WebDriver server and the test runner using `xvfb-run`. We explicitly set the virtual screen resolution to ensure the app renders correctly before taking screenshots.

```bash
# 1. Start the tauri webdriver server inside the virtual display
xvfb-run --auto-servernum --server-args="-screen 0 1280x800x24" tauri-wd --port 4444 > tauri-wd.log 2>&1 &

# 2. Wait a few seconds for the server to spin up
sleep 3

# 3. Run the tests inside the virtual display
xvfb-run --auto-servernum --server-args="-screen 0 1280x800x24" npx wdio run wdio.conf.mjs
```

Screenshots taken during the test run will be saved to `e2e-tests/screenshots/`.

---

## 4. Troubleshooting

### Screenshots show no styles (plain unstyled HTML)

**Symptom:** Test screenshots in `e2e-tests/screenshots/` look like raw, unstyled HTML — no colours, no layout, no Tailwind classes applied — even though the running app looks correct.

**Root cause:** The e2e test suite launches the compiled native binary (`src-tauri/target/debug/harvey`) directly via `tauri-wd`. That binary has the entire frontend (`build/`) **embedded inside it at compile time** via Tauri's `frontendDist` setting. If the binary was compiled before (or without) running `npm run build`, it contains an old version of the frontend with no up-to-date CSS.

This is fundamentally different from `tauri dev`, which serves a live Vite dev server on port 1420 that always injects the latest styles. E2e tests never hit that dev server.

**Fix:** Always run `npm run tauri build -- --debug` before executing the test suite. See [Step 1 above](#step-1--build-the-debug-binary) for details.

---

## 5. How It Works (Bypassing System Dialogs)

Web automation tools like WebdriverIO cannot interact with native OS-level file explorer windows (e.g., macOS Finder or Windows Explorer).

When a user clicks "Create Project" in Harvey, the app typically calls Tauri's `saveDialog` plugin. To bypass this during E2E testing without modifying the Rust backend, the frontend `actions.js` listens for a specific `window` variable.

In the E2E script (`full-flow.e2e.js`), we inject this variable before triggering the click:
```javascript
await browser.execute((path) => {
    window.__E2E_TEST_PROJECT_PATH__ = path;
}, dummyPath);
```
The application detects this flag and instantly returns the dummy path, skipping the native dialog entirely and allowing the test flow to continue uninterrupted.

---

## 6. Reference Screenshots

The `e2e-tests/reference-screenshots/` directory contains known-good baseline screenshots captured from a correctly-built binary. If your test screenshots in `e2e-tests/screenshots/` look visually wrong, compare them against the reference set to identify regressions. Update the reference screenshots after any intentional UI change by copying the passing screenshots over.