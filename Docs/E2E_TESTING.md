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
> If you skip this step, the test logs may reflect stale application logic or missing assets.

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
npx wdio run wdio.conf.mjs
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
Wrap both the WebDriver server and the test runner using `xvfb-run`.

```bash
# 1. Start the tauri webdriver server inside the virtual display
xvfb-run --auto-servernum --server-args="-screen 0 1280x800x24" tauri-wd --port 4444 > tauri-wd.log 2>&1 &

# 2. Wait a few seconds for the server to spin up
sleep 3

# 3. Run the tests inside the virtual display
xvfb-run --auto-servernum --server-args="-screen 0 1280x800x24" npx wdio run wdio.conf.mjs
```

Console logs captured during the test run will be saved to `e2e-tests/logs/`.

---

## 4. Troubleshooting

### Logs show unexpected errors or stale behavior

**Symptom:** The captured logs in `e2e-tests/logs/` show incorrect application state or errors that don't appear in the live `tauri dev` environment.

**Root cause:** The e2e test suite launches the compiled native binary (`src-tauri/target/debug/harvey`) directly via `tauri-wd`. That binary has the entire frontend (`build/`) **embedded inside it at compile time** via Tauri's `frontendDist` setting. If the binary was compiled before (or without) running `npm run build`, it contains an old version of the frontend.

**Fix:** Always run `npm run tauri build -- --debug` before executing the test suite.

---

## 5. How It Works (Bypassing System Dialogs and Capturing Logs)

### Bypassing Native Dialogs
Web automation tools like WebdriverIO cannot interact with native OS-level file explorer windows. In the E2E script (`full-flow.e2e.js`), we inject a variable into the window before triggering a project creation:
```javascript
await browser.execute((path) => {
    window.__E2E_TEST_PROJECT_PATH__ = path;
}, dummyPath);
```

### Capturing Console Logs
Since stdout from the Tauri webview is not always easily accessible across all platforms via WebDriver, the test suite injects a capture script that monkey-patches `console.log`, `console.error`, etc., to store logs in a global array. At each test stage, these logs are retrieved and saved to files in `e2e-tests/logs/`.
hots over.