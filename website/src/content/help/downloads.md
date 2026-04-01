---
id: downloads
label: Downloads
sidebarId: overview
order: 3
---

Visit the [Latest Releases](https://github.com/Ethnomethodology/harvey/releases/tag/v0.1.1) page on GitHub to download Harvey for your platform.

## Windows
Download the file ending in `x64-setup.zip`. Extract the content and run the setup executable to install Harvey on your system.

> **Note on Security Warnings:** As an open-source project, Harvey is not yet digitally signed by a commercial certificate authority. If Windows SmartScreen displays a warning, you can safely proceed by clicking **"More info"** and then **"Run anyway"**.

## macOS
1.  **Download**: Visit the [GitHub Releases](https://github.com/Ethnomethodology/harvey/releases/tag/v0.1.1) page.
    - For **Apple Silicon (M1, M2, etc.)**: Download the file with `aarch64` in the name.
    - For **Intel Macs**: Download the file with `x64` in the name.
2.  **Install**: Open the downloaded `.dmg` file and drag `harvey.app` to your `/Applications` folder.
3.  **Security Warnings**: As an open-source project, Harvey is not yet digitally signed by a commercial certificate authority. Because the app is not currently signed, macOS will prevent it from running. To authorize it after installation, open the **Terminal** application and run the following commands:
    ```bash
    cd /Applications
    sudo xattr -dr com.apple.quarantine harvey.app
    ```

## Build from Source
If you are using Linux or prefer to build the application yourself, we provide a bootstrap script that automates the environment setup and compilation.

### Quick Build (macOS & Linux)
Run the following command in your terminal to check for dependencies and build Harvey automatically:

```bash
curl -sSL https://raw.githubusercontent.com/Ethnomethodology/harvey/main/scripts/bootstrap.sh | bash
```

> [!NOTE]
> This script will verify your system has Git, Node.js, and Rust installed. On Linux, it also checks for required development libraries. If anything is missing, it will provide installation instructions.

### Manual Build
Alternatively, you can clone the repository and run the script locally:
1.  **Clone**: `git clone https://github.com/Ethnomethodology/harvey.git`
2.  **Run Bootstrapper**: `cd harvey && bash scripts/bootstrap.sh`

Refer to the `README.md` for manual build instructions if you prefer a custom setup.
