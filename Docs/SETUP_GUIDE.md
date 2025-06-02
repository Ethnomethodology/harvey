# Project Harvey 1.0 - Setup Guide

Welcome to the Project Harvey 1.0 setup guide! This document will walk you through the steps to get the Project Harvey 1.0 application up and running on your computer. We'll aim to keep things as clear and simple as possible!

## Part 1: Getting Started with Essential Tools

Before you can work with Project Harvey, you'll need a few common tools. These tools will help you download the project's code, manage it, and run it.

### 1.1. GitHub Desktop (Recommended for Beginners)

GitHub is a website where many software projects, including Project Harvey, store their code. GitHub Desktop is a user-friendly application that makes it easy to download and manage code from GitHub without needing to use complex commands.

*   **What it's for**: Downloading and managing the project's source code.
*   **How to get it**:
    1.  Go to the [GitHub Desktop official website](https://desktop.github.com/).
    2.  Download the installer for your operating system (Windows or macOS).
    3.  Run the installer and follow the on-screen instructions. You may be asked to create a free GitHub account if you don't have one.

### 1.2. Visual Studio Code (VSCode)

Visual Studio Code (often called VSCode) is a very popular, free code editor. While you generally won't need to write or modify code to *use* Project Harvey 1.0 once it's released, VSCode can be helpful for viewing project files if you're following this guide to set up the development version. It also has good integration with Git and GitHub Desktop.

*   **What it's for**: Viewing project files; can also help manage code if you become more advanced.
*   **How to get it**:
    1.  Go to the [Visual Studio Code official website](https://code.visualstudio.com/).
    2.  Download the installer for your operating system (Windows, macOS, or Linux).
    3.  Run the installer and follow the on-screen instructions.

### 1.3. Git (Optional, if using GitHub Desktop)

Git is the underlying version control system that GitHub Desktop uses. If you install GitHub Desktop, it usually includes its own version of Git, or it will prompt you to install it. For most non-technical users, GitHub Desktop is enough.

*   **What it's for**: The powerful but more command-line focused tool for code management that GitHub Desktop uses.
*   **How to get it (if you want it separately or are curious)**:
    1.  Go to the [Git official website](https://git-scm.com/downloads).
    2.  Download and run the installer for your system.
    *   **Note for beginners**: The installation options for Git can be numerous and confusing. If you're new to this, sticking with GitHub Desktop is much simpler, as it handles Git for you.

---
## Part 2: Getting the Project Code

Now that you have the essential tools, let's get the Project Harvey code onto your computer. This process is often called "cloning a repository."

### 2.1. Using GitHub Desktop (Recommended)

1.  **Open GitHub Desktop**: Launch the application.
2.  **Clone a Repository from the Internet**:
    *   If it's your first time, you might see a "Let's get started!" screen. Choose "Clone a repository from the Internet..."
    *   If you've used it before, go to `File > Clone Repository...` from the top menu.
3.  **Select the URL Tab**: In the "Clone a Repository" dialog, make sure the "URL" tab is selected.
4.  **Enter the Repository URL**:
    *   The URL for Project Harvey's code is: `https://github.com/your-username/project-harvey.git`
        *   **(Developer Note: Please replace `your-username/project-harvey.git` with the actual URL of the Project Harvey repository when this documentation is finalized. For now, this placeholder is used.)**
    *   Paste this URL into the "Repository URL or GitHub CLI command" field.
5.  **Choose a Local Path**:
    *   The "Local path" field shows which folder on your computer the project code will be saved into. GitHub Desktop usually suggests a folder like `Documents/GitHub/project-harvey`. You can change this if you prefer, but the default is often fine. Make a note of this location!
6.  **Click "Clone"**: GitHub Desktop will now download all the project files to the folder you specified. This might take a few minutes depending on your internet speed.

### 2.2. Using Visual Studio Code (Alternative)

If you prefer, and are comfortable with VSCode, it can also clone repositories.

1.  **Open VSCode**: Launch the application.
2.  **Open the Command Palette**:
    *   Go to `View > Command Palette...` from the top menu (or press `Ctrl+Shift+P` on Windows/Linux, `Cmd+Shift+P` on macOS).
3.  **Type `Git: Clone`**: In the Command Palette, start typing `Git: Clone` and select it from the list when it appears.
4.  **Enter the Repository URL**:
    *   Paste the Project Harvey repository URL: `https://github.com/your-username/project-harvey.git`
        *   **(Important Developer Note: The URL above is a placeholder! This must be replaced with the actual URL of the Project Harvey GitHub repository when this documentation is finalized.)**
    *   Press `Enter`.
5.  **Select a Folder (Local Path)**: VSCode will ask you to choose an existing folder on your computer where you want to save the project files. Select or create a suitable folder and then click "Select Repository Location".
6.  **Open the Cloned Repository**: Once cloning is complete, VSCode will usually show a notification asking if you want to open the cloned repository. Click "Open".

After this step, you should have a copy of the Project Harvey 1.0 code in a folder on your computer!

---
## Part 3: Setting Up the Backend (Rust and Tauri)

Project Harvey's backend (the core engine) is built with Rust and Tauri. These need to be set up before the application can run. This part can be a bit more technical, so take your time and follow the steps carefully.

### 3.1. Install Rust

Rust is a programming language. You'll need to install it using a tool called `rustup`.

1.  **Open a Terminal (Command Prompt)**:
    *   **Windows**: Search for `cmd` or `PowerShell` in the Start Menu.
    *   **macOS**: Open `Terminal` (you can find it in `Applications > Utilities`).
    *   **Linux**: Open your distribution's terminal (e.g., `Terminal`, `Konsole`).
2.  **Go to the `rustup` website**: Visit [https://rustup.rs/](https://rustup.rs/).
3.  **Follow Instructions**: The website will show you a command to copy and paste into your terminal.
    *   It usually looks something like `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh` for macOS/Linux, and provides a downloadable `rustup-init.exe` for Windows.
    *   Run the command or the downloaded installer.
4.  **Choose Default Installation**: When prompted by the installer in the terminal, just press `Enter` to choose the default installation options.
5.  **Restart Your Terminal**: After installation is complete, close your current terminal window and open a brand new one. This is important because it allows your system to recognize the new Rust commands (by refreshing its PATH environment variable).
6.  **Verify Installation (Optional)**: In the new terminal, type `rustc --version` and press `Enter`. If Rust is installed correctly, you'll see its version number printed.

### 3.2. Install Tauri System Prerequisites (Very Important!)

Tauri applications rely on your system having certain "webview" software (which is like a built-in web browser component that Tauri uses to display the user interface) and other development tools. These prerequisites are different for each operating system.

*   **This is the most crucial step for the backend setup.**
*   **Go to the Official Tauri Prerequisites Guide**: [https://tauri.app/v1/guides/getting-started/prerequisites](https://tauri.app/v1/guides/getting-started/prerequisites)
*   **Carefully follow the instructions for YOUR specific operating system**:
    *   **Windows**: Usually involves installing Microsoft Visual Studio C++ Build Tools. The guide has specific checkboxes to select during that installation.
    *   **macOS**: Typically requires Xcode Command Line Tools.
    *   **Linux**: Requires several packages like `libwebkit2gtk-4.0-dev`, `build-essential`, etc. The guide lists them per distribution (like Ubuntu, Fedora).
*   **Take your time with this step.** If these prerequisites are not met, the project will not build.

### 3.3. Install the Tauri CLI (Command Line Interface)

The Tauri CLI is a tool that helps manage and build Tauri applications.

1.  **Open a Terminal** (a new one if you just finished Rust installation or prerequisites).
2.  **Navigate to the Project Folder (Good Practice)**:
    *   It's good practice to run installation commands from within the project folder if possible, though for global tools like `tauri-cli` it's not strictly necessary.
    *   In the terminal, you can navigate to where you cloned Project Harvey 1.0. For example, if it's in `Documents/GitHub/project-harvey`:
        *   Type `cd Documents/GitHub/project-harvey` and press `Enter`. The `cd` command means "change directory." This tells your terminal to switch its focus to that specific folder.
3.  **Install Tauri CLI**: Type the following command into the terminal and press `Enter`:
    ```bash
    cargo install tauri-cli
    ```
    *   This command uses `cargo` (Rust's package manager, which was installed with Rust) to download and install the `tauri-cli` tool. This might take a few minutes.
4.  **Verify Installation (Optional)**: In the terminal, type `cargo tauri --version` and press `Enter`. If it's installed correctly, you'll see its version number.
    *   *(Note: If the `cargo tauri` command isn't found, you might need to restart your terminal, or in rare cases, your computer. This helps ensure the system recognizes the new path where `cargo` installs command-line tools.)*

Once these steps are done, your system should be ready for the Project Harvey 1.0 backend!

---
## Part 4: Setting Up the Frontend (Node.js and Dependencies)

Project Harvey's frontend (the user interface you see and interact with) is built using web technologies and requires Node.js.

### 4.1. Install Node.js and npm

Node.js is a JavaScript runtime environment that lets you run JavaScript code outside of a web browser. npm (Node Package Manager) is included with Node.js and is used to manage project dependencies (like SvelteKit and other libraries the frontend needs).

1.  **Go to the Node.js Website**: Visit [https://nodejs.org/](https://nodejs.org/).
2.  **Download the LTS Version**: You'll usually see two download options: LTS (Long-Term Support) and Current. It's generally recommended to download the **LTS** version, as it's more stable.
3.  **Run the Installer**: Once downloaded, run the installer for your operating system (Windows, macOS, or Linux).
4.  **Follow Installation Prompts**: Accept the license agreement and follow the on-screen instructions, choosing the default options. npm will be installed automatically with Node.js.
5.  **Verify Installation (Optional)**:
    *   Open a **new** terminal or command prompt.
    *   Type `node -v` and press `Enter`. You should see the Node.js version.
    *   Type `npm -v` and press `Enter`. You should see the npm version.

### 4.2. Install Frontend Dependencies

Now that you have Node.js and npm, you can install all the specific libraries and packages Project Harvey's frontend needs.

1.  **Open a Terminal (or use VSCode's built-in terminal)**.
2.  **Navigate to the Project Harvey Folder**:
    *   If your terminal isn't already in the Project Harvey 1.0 folder (e.g., `Documents/GitHub/project-harvey`), use the `cd` command (which means "change directory") to navigate there. For example:
        ```bash
        cd path/to/your/project-harvey-folder
        ```
        (Replace `path/to/your/project-harvey-folder` with the actual path on your computer where you cloned the project. This needs to be the main project folder that contains the `src-tauri` folder and other files like `package.json`.)
    *   **Tip**: If you have the project open in VSCode, you can open an integrated terminal by going to `Terminal > New Terminal` in the VSCode menu. It will usually start in the correct project folder automatically.
3.  **Run `npm install`**:
    *   Once your terminal is focused on the root directory of the Project Harvey 1.0 project (this is the folder that contains a file named `package.json`), type the following command and press `Enter`:
        ```bash
        npm install
        ```
    *   **What this does**: This command reads the `package.json` file, which lists all the frontend libraries (or "packages") that Project Harvey 1.0 needs to work. `npm` then downloads these packages from the internet and installs them into a folder called `node_modules` within your project directory.
    *   This step can take a few minutes, and you'll see a lot of text scrolling in the terminal as files are downloaded and installed. Don't worry unless you see prominent `ERROR` messages at the very end. General warnings (`WARN`) are usually okay.

After this, the frontend part of Project Harvey 1.0 should be all set up!

---
## Part 5: Building and Running the Project

With both the backend and frontend dependencies set up, you're ready to run Project Harvey!

There are two main ways to run the application:

1.  **Development Mode**: This is what you'll usually use. It starts the application with special tools that help developers, like automatic reloading when code changes. It might be a bit slower to start but is great for development or just running the latest code.
2.  **Production Build**: This creates a more optimized, standalone version of the application, like what you'd download if it were officially released.

For just running the application on your computer after setup, development mode is perfectly fine.

### 5.1. Running in Development Mode

1.  **Open a Terminal**.
2.  **Navigate to the Project Harvey Folder**:
    *   Make sure your terminal is in the root directory of Project Harvey 1.0 (the main folder containing all project files, e.g., `cd path/to/your/project-harvey-folder`).
3.  **Run the Development Command**:
    *   Type the following command and press `Enter`:
        ```bash
        npm run tauri dev
        ```
    *   Alternatively, you can often use this command which does the same thing:
        ```bash
        cargo tauri dev
        ```
    *   **What happens now?**
        *   This command tells Tauri to build both the frontend and the backend code and then launch the application in "development mode."
        *   You'll see a lot of output in your terminal. The very first time you run this, it might take several minutes because Rust needs to compile all the backend code (you'll see messages about "compiling crates" – "crates" are what Rust calls its packages or libraries). Subsequent builds are usually much faster.
        *   If everything is set up correctly, after the build process finishes, the Project Harvey 1.0 application window should automatically open on your screen!

    *   **Troubleshooting Tips**:
        *   **Correct Folder**: Make sure your terminal is definitely in the main Project Harvey 1.0 folder before running the command.
        *   **Backend Errors**: If you see errors mentioning "Rust," "cargo," or "Tauri" specifically, double-check that you completed all steps in "Part 3: Setting Up the Backend" correctly. The Tauri system prerequisites (Step 3.2) are especially common culprits if missed.
        *   **Frontend Errors**: If errors seem related to "npm," "Node.js," or specific JavaScript packages, review "Part 4" to ensure Node.js is installed and `npm install` completed without major errors.

### 5.2. Creating a Production Build (Optional)

If you want to create a standalone application package (like an `.exe` file on Windows, an `.app` file on macOS, or an executable file on Linux that you could share with others who don't have all these development tools), you can create a "production build."

1.  **Open a Terminal**.
2.  **Navigate to the Project Harvey Folder** (the main root directory).
3.  **Run the Build Command**:
    *   Type the following command and press `Enter`:
        ```bash
        npm run tauri build
        ```
    *   Or, alternatively:
        ```bash
        cargo tauri build
        ```
    *   This process will also take some time. Once it's finished, it will usually tell you where the built application files are located. This is typically in a folder like `src-tauri/target/release/bundle/` within your project directory.

You've now successfully run Project Harvey 1.0! If you used the `dev` command, you can close the application window. To stop the development server that's running in your terminal, you can usually press `Ctrl+C` in the terminal window.

---
## Part 6: How to Request a Feature or Report a Bug

Your feedback is valuable for making Project Harvey better! If you have an idea for a new feature, or if you encounter a problem (a "bug"), here’s how you can let the developers know through GitHub.

### 6.1. Understanding GitHub Issues

GitHub Issues is a tracking system built into every GitHub repository. It's used to track tasks, enhancements, and bugs for projects.

*   **Where to find it**:
    1.  Go to the Project Harvey 1.0 repository page on GitHub in your web browser (this is the same URL you used for cloning, e.g., `https://github.com/your-username/project-harvey.git` - **Important Developer Note: This URL is a placeholder and needs to be replaced with the actual project URL!**).
    2.  Look for a tab at the top of the repository page that says "Issues." Click on it.

### 6.2. Before Creating a New Issue

*   **Search Existing Issues**: Someone might have already reported the same bug or suggested a similar feature. Use the search bar within the "Issues" tab (you can filter by keywords or labels) to see if there's an existing discussion.
    *   If you find a similar *open* issue, you can add your comments or reactions to it instead of creating a new one.
    *   If you find a similar *closed* issue, it might have a solution or explanation.

### 6.3. Creating a New Issue

If your bug or feature idea seems new:

1.  **Click the "New Issue" Button**: On the "Issues" page, you'll see a green "New issue" button. Click it.
2.  **Choose a Template (if available)**: Some projects have templates for bug reports or feature requests. If you see options, choose the one that best fits what you want to submit.
3.  **Write a Clear Title**: Make the title concise and descriptive.
    *   **Good Bug Title Example**: "Application crashes when importing MP3 files over 1GB"
    *   **Good Feature Title Example**: "Suggestion: Add option to export transcript as plain text (.txt)"
4.  **Describe the Issue in Detail (Leave a Comment)**:
    *   The main text area is where you provide details. The more information you give, the better the developers can understand and help.

    *   **For Bug Reports, try to include**:
        *   **What you did**: Step-by-step actions you took leading up to the problem.
        *   **What you expected to happen**: The desired outcome.
        *   **What actually happened**: Describe the error or incorrect behavior. Include any error messages you saw (you can copy and paste them).
        *   **Your computer's operating system**: (e.g., Windows 10, macOS Ventura, Ubuntu 22.04).
        *   **Project Harvey 1.0 Version (if known)**: If you built from specific code or know a version number, include it.
        *   **Screenshots (if helpful)**: You can often drag and drop or attach images directly into the comment box on GitHub to upload them. A picture can often explain a problem much faster!

    *   **For Feature Requests, try to include**:
        *   **What problem does this feature solve?** Or, what new capability would it enable for users?
        *   **How would you ideally like it to work?** Describe your vision for the feature.
        *   **Are there any alternatives you've considered or seen in other software?** (Optional)

5.  **Use Markdown for Formatting**: You can use simple text formatting called Markdown (like `## Headings`, `* bullet points`, `` `code` `` for code snippets or commands, `**bold text**`) to make your issue easier to read. GitHub provides a formatting toolbar above the text area as well.
6.  **Submit the Issue**: Once you're satisfied with your report or request, click the "Submit new issue" button.

A project maintainer will review your issue and respond when they can. Your contributions, whether bug reports or feature ideas, are highly appreciated and help make Project Harvey 1.0 better for everyone!

Thank you for following this setup guide!
