## Agent Setup Guide for Google's Jules (Linux Environment)

This guide outlines the steps to set up and run the Harvey application and its Playwright tests within a Linux environment, such as Google's Jules.

### 1. System Dependencies (Tauri Prerequisites)

Ensure your Linux environment has the necessary Tauri prerequisites installed. These typically include:

*   **System build tools:** `gcc`, `pkg-config`, `libssl-dev`, `libwebkit2gtk-4.0-dev`, `libappindicator3-dev`, `librsvg2-dev`, `patchelf`.
*   **Node.js and npm/pnpm:** Ensure a compatible version is installed.

For the most up-to-date and detailed list of prerequisites for your specific Linux distribution, please refer to the official Tauri documentation:
[https://tauri.app/v1/guides/getting-started/prerequisites/#setting-up-linux](https://tauri.app/v1/guides/getting-started/prerequisites/#setting-up-linux)

### 2. Install `tauri-driver`

`tauri-driver` is essential for running Playwright tests with your Tauri application. Install it using Cargo:

```bash
cargo install tauri-driver --locked
```

### 3. Install Native WebDriver (WebKitWebDriver)

Tauri leverages `WebKitWebDriver` on Linux for WebDriver testing. Verify its presence or install it:

*   **Check if installed:**
    ```bash
    which WebKitWebDriver
    ```

*   **If not found (e.g., on Debian/Ubuntu-based systems):**
    ```bash
    sudo apt update
    sudo apt install webkit2gtk-driver
    ```
    Ensure `WebKitWebDriver` is accessible in your system's `$PATH`.

### 4. Project Setup

Navigate to the project's root directory and install the JavaScript dependencies:

```bash
npm install
```

### 5. Running Tests

To execute the Playwright tests, use the following command from the project root:

```bash
npm test tests/verify_project_creation.spec.js
```

### 6. Running the Application

To run the application in development mode:

```bash
npm run tauri dev
```

To build and run the production application:

```bash
npm run build
npm run tauri build
# The executable will be located in src-tauri/target/release/harvey (or similar, depending on your OS and build configuration)
# You can then run it directly:
# ./src-tauri/target/release/harvey
```