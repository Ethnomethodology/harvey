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

Before running tests, ensure you have compiled a debug build of the application:
```bash
npm run tauri build -- --debug
```

### macOS & Linux
1. Open a terminal and start the Tauri WebDriver server in the background:
   ```bash
   tauri-wd --port 4444 &
   ```
2. Run the WebdriverIO test suite:
   ```bash
   npx wdio run wdio.conf.mjs
   ```

### Windows
1. Open PowerShell or Command Prompt and start the Tauri WebDriver server:
   ```powershell
   Start-Process tauri-wd -ArgumentList "--port 4444" -NoNewWindow
   ```
2. Run the WebdriverIO test suite:
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

## How It Works (Bypassing System Dialogs)

Web automation tools like WebdriverIO cannot interact with native OS-level file explorer windows (e.g., macOS Finder or Windows Explorer).

When a user clicks "Create Project" in Harvey, the app typically calls Tauri's `saveDialog` plugin. To bypass this during E2E testing without modifying the Rust backend, the frontend `actions.js` listens for a specific `window` variable.

In the E2E script (`full-flow.e2e.js`), we inject this variable before triggering the click:
```javascript
await browser.execute((path) => {
    window.__E2E_TEST_PROJECT_PATH__ = path;
}, dummyPath);
```
The application detects this flag and instantly returns the dummy path, skipping the native dialog entirely and allowing the test flow to continue uninterrupted.