# Project Harvey 1.0 - Setup Guide

Welcome to the Project Harvey 1.0 setup guide! This document will walk you through the steps to get the Project Harvey 1.0 application up and running on your computer. We'll aim to keep things as clear and simple as possible!

**(Note: When typing commands in the terminal sections of this guide, they are often case-sensitive, meaning 'MyFolder' is different from 'myfolder'. It's best to type them exactly as shown.)**

## Part 1: Getting Started with Essential Tools

Before you can work with Project Harvey, you'll need a few common tools. These tools will help you download the project's code (the 'source code' or set of instructions that make the application work), manage it, and run it.

### 1.1. GitHub Desktop (Recommended for Beginners)

GitHub is a website where many software projects, including Project Harvey, store their code. GitHub Desktop is a user-friendly application that makes it easy to download and manage code from GitHub without needing to use complex commands.

*   **What it's for**: Downloading and managing the project's source code.
*   **How to get it**:
    1.  Go to the [GitHub Desktop official website](https://desktop.github.com/).
    2.  Download the installer for your operating system (Windows or macOS).
    3.  Run the installer. (You'll typically find this downloaded file, e.g., `GitHubDesktopSetup.exe` on Windows or `GitHubDesktop.zip` on macOS, in your computer's 'Downloads' folder. Double-click it to start.) Follow the on-screen instructions. You may be asked to create a free GitHub account if you don't have one; this is free and recommended.

### 1.2. Visual Studio Code (VSCode)

Visual Studio Code (often called VSCode) is a very popular, free code editor. While you generally won't need to write or modify code to *use* Project Harvey 1.0 once it's released, VSCode can be helpful for viewing project files if you're following this guide to set up the development version. It also has good integration with Git and GitHub Desktop.

*   **What it's for**: Viewing project files; can also help manage code if you become more advanced.
*   **How to get it**:
    1.  Go to the [Visual Studio Code official website](https://code.visualstudio.com/).
    2.  Download the installer for your operating system (Windows, macOS, or Linux). (Look for a prominent button that says 'Download' for your OS.)
    3.  Run the installer. (You'll typically find this downloaded file, e.g., `VSCodeUserSetup.exe` on Windows or a `.dmg` file on macOS, in your 'Downloads' folder. Double-click it to start.) Follow the on-screen instructions; accepting the default options is usually fine.

### 1.3. Git (Optional, if using GitHub Desktop)

Git is the powerful tool working behind the scenes that GitHub Desktop uses to manage changes to code over time (this is often called 'version control'). If you install GitHub Desktop, it usually includes its own version of Git, or it will prompt you to install it. For most non-technical users, GitHub Desktop is enough.

*   **What it's for**: The more command-line focused tool for code management that GitHub Desktop uses.
*   **How to get it (if you want it separately or are curious)**:
    1.  Go to the [Git official website](https://git-scm.com/downloads).
    2.  Download and run the installer for your system. (You'll typically find this in your 'Downloads' folder.)
    *   **Note for beginners**: The installation options for Git can be numerous and confusing (e.g., it might ask about PATH settings, default editors). If you're new to this, sticking with GitHub Desktop is much simpler, as it handles Git for you. Choosing the default options during the Git install is generally safe if you do proceed.

---
## Part 2: Getting the Project Code

Now that you have the essential tools, let's get the Project Harvey code onto your computer. This process is often called "cloning a repository" (a "repository" or "repo" is just what a project's collection of code and files is called on GitHub).

### 2.1. Using GitHub Desktop (Recommended)

1.  **Open GitHub Desktop**: Launch the application.
2.  **Clone a Repository from the Internet**:
    *   If it's your first time, you might see a "Let's get started!" screen. Look for a prominent button, often labeled "Clone a repository from the Internet..." or similar.
    *   If you've used it before, go to `File > Clone Repository...` from the top menu.
3.  **Select the URL Tab**: In the "Clone a Repository" dialog that appears, make sure the "URL" tab is selected.
4.  **Enter the Repository URL**:
    *   The URL for Project Harvey's code is: `https://github.com/Ethnomethodology/harvey`
    *   Paste this URL into the "Repository URL or GitHub CLI command" field.
5.  **Choose a Local Path**:
    *   The "Local path" field shows which folder on your computer the project code will be saved into. GitHub Desktop will suggest a default folder, often within your user's `Documents` directory (e.g., `Documents/GitHub/project-harvey` or similar). You can change this if you prefer by clicking "Choose...", but the default is often fine. Make a note of this location!
6.  **Click "Clone"**: GitHub Desktop will now download all the project files to the folder you specified. This might take a few minutes depending on your internet speed. Once it's done, you should see the project appear in GitHub Desktop, and the files will be in the local path you chose.

### 2.2. Using Visual Studio Code (Alternative)

If you prefer, and are comfortable with VSCode, it can also clone repositories.

1.  **Open VSCode**: Launch the application.
2.  **Open the Command Palette**:
    *   Go to `View > Command Palette...` from the top menu (or press `Ctrl+Shift+P` on Windows/Linux, `Cmd+Shift+P` on macOS). A search bar will appear at the top of the VSCode window.
3.  **Type `Git: Clone`**: In the Command Palette search bar, start typing `Git: Clone` and select it from the list when it appears (then press `Enter` or `Return`).
4.  **Enter the Repository URL**:
    *   Paste the Project Harvey repository URL: `https://github.com/Ethnomethodology/harvey`
    *   Press `Enter` or `Return`.
5.  **Select a Folder (Local Path)**: VSCode will ask you to choose an existing folder on your computer where you want to save the project files. Select or create a suitable folder and then click "Select Repository Location".
6.  **Open the Cloned Repository**: Once cloning is complete, VSCode will usually show a notification asking if you want to open the cloned repository. Click "Open". VSCode will then load all the project files, and you'll see them listed in the 'Explorer' panel on the left side.

After this step, you should have a copy of the Project Harvey 1.0 code in a folder on your computer!

---
## Part 3: Setting Up the Backend (Rust and Tauri)

Project Harvey's backend (the core engine, which works behind the scenes) is built with Rust and Tauri. These need to be set up before the application can run. This part can be a bit more technical, so take your time and follow the steps carefully.

### 3.1. Install Rust

Rust is a programming language. You'll need to install it using a tool called `rustup`.

1.  **Open a Terminal (Command Prompt)**:
    *   A terminal (also known as a command prompt or command line) is a text-based way to interact with your computer. It's usually a black or dark-themed window where you can type commands directly.
    *   **Windows**: Search for `cmd` or `PowerShell` in the Start Menu and click on it.
    *   **macOS**: Open `Terminal` (you can find it in `Applications > Utilities`).
    *   **Linux**: Open your distribution's terminal (e.g., `Terminal`, `Konsole`).
2.  **Go to the `rustup` website**: Visit [https://rustup.rs/](https://rustup.rs/).
3.  **Follow Instructions**: The website will show you a command to copy and paste into your terminal.
    *   It usually looks something like `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh` for macOS/Linux. For Windows, it usually provides a downloadable `rustup-init.exe` file.
    *   **For macOS/Linux**: Copy the `curl...` command, paste it into your terminal window, and press `Enter` or `Return` to run it.
    *   **For Windows**: Download `rustup-init.exe`. You'll typically find this in your 'Downloads' folder. Navigate there using your File Explorer and double-click `rustup-init.exe` to start its installation (it will open a terminal-like window).
4.  **Choose Default Installation**: The installer will ask you to choose installation options (e.g., "1) Proceed with installation (default)"). For most users, the default option is suitable. Press `Enter` or `Return` (or type `1` then `Enter`/`Return` if prompted) to continue.
5.  **Restart Your Terminal**: After installation is complete (you should see a message like "Rust is installed now. Great!"), close your current terminal window and open a brand new one. This is important because it allows your system to recognize the new Rust commands.
    *   (Tip: If a command isn't recognized later, try closing your terminal window and opening a brand new one. This often helps your system find newly installed programs.)
6.  **Verify Installation (Optional)**: In the new terminal, type `rustc --version` (and press `Enter` or `Return`). If Rust is installed correctly, you'll see its version number printed (e.g., `rustc 1.70.0`). If you see an error message like 'command not found,' try the tip in the previous step, or ensure the installation in step 3 & 4 completed without errors.

### 3.2. Install Tauri System Prerequisites (Very Important!)

Tauri 2.0 applications rely on your system having certain "webview" software (which is like a built-in web browser component that Tauri uses to display the user interface) and other development tools. These prerequisites are different for each operating system.

*   **This is the most crucial step for the backend setup for Tauri 2.0.**
*   **Go to the Official Tauri V2 Prerequisites Guide**: [https://v2.tauri.app/develop/guides/prerequisites/](https://v2.tauri.app/develop/guides/prerequisites/)
    *   (Tip: The Tauri prerequisites page can look a bit technical. Focus only on the section for your specific Operating System: Windows, macOS, or Linux. You can ignore the sections for other systems.)
*   **Carefully follow the instructions for YOUR specific operating system**:
    *   **Windows**: Usually involves installing Microsoft Visual Studio C++ Build Tools (ensure you select the correct workloads and components as specified in the Tauri guide). Pay very close attention to selecting the exact components mentioned, as missing one can cause build issues later.
    *   **macOS**: Typically requires Xcode Command Line Tools.
    *   **Linux**: Requires several packages like `webkit2gtk-4.1-dev` (or similar, depending on distribution and specific GTK version), `build-essential`, etc. The Tauri guide lists them per distribution (like Ubuntu, Fedora).
*   **Take your time with this step and ensure all components listed for your OS in the Tauri v2 guide are installed.** If these prerequisites are not met, the project will not build.

### 3.3. Understanding the Tauri CLI (Project-Managed)

The Tauri CLI (Command Line Interface) is a tool that helps manage and build Tauri applications.

For Project Harvey 1.0, the Tauri CLI tool is already listed as a specific tool for the project in a file called `package.json` (this file helps manage project tools and libraries; you'd see an entry like `@tauri-apps/cli` in that file if you were to look). This is called a **development dependency**.

*   **No Global Installation Needed**: You **do not** need to install `tauri-cli` globally on your system using a command like `cargo install tauri-cli`.
*   **How it Works**: When you run Tauri commands using `npm run tauri ...` (for example, `npm run tauri dev` or `npm run tauri build`, which will be covered in Part 5), `npm` automatically uses the specific version of the Tauri CLI that is included in the project's dependencies.
*   This approach ensures that all developers (and you, when setting up) use the same version of the CLI, which helps avoid compatibility issues.

So, there's no separate installation step for the Tauri CLI itself beyond what `npm install` (covered in Part 4) will handle for project-specific tools.

Once the Rust setup (3.1) and Tauri System Prerequisites (3.2) are successfully completed, your system should be ready for Project Harvey 1.0's backend!

---
## Part 4: Setting Up the Frontend (Node.js and Dependencies)

Project Harvey's frontend (the user interface you see and interact with) is built using SvelteKit (with JavaScript) and other web technologies, all of which require Node.js. "Dependencies" are extra code libraries the project needs to work.

### 4.1. Install Node.js and npm

Node.js is a tool that allows JavaScript (the programming language of the web) to run on your computer outside of a web browser. It's needed for many web development tools. npm (Node Package Manager) comes with Node.js and helps manage all the extra code libraries (often called 'dependencies' or 'packages') that Project Harvey's user interface needs.

1.  **Go to the Node.js Website**: Visit [https://nodejs.org/](https://nodejs.org/).
2.  **Download the LTS Version**: You'll usually see two download options: LTS (Long-Term Support) and Current. It's generally recommended to download the **LTS** version, as it's more stable. Look for a button that clearly indicates "LTS".
3.  **Run the Installer**: Once downloaded (you'll typically find this file, e.g., `node-vXX.X.X-lts.msi` for Windows or a `.pkg` file for macOS, in your 'Downloads' folder), double-click the installer file for your operating system to start it.
4.  **Follow Installation Prompts**: Accept the license agreement and follow the on-screen instructions. During installation, you'll be asked several questions. For most users, accepting the **default options** at each step is the best choice (this usually means just clicking 'Next' or 'Continue' without changing any pre-selected checkboxes or settings). Ensure that the option to install 'npm package manager' is selected (it usually is by default).
5.  **Verify Installation (Optional)**:
    *   Open a **new** terminal or command prompt.
    *   Type `node -v` (and press `Enter` or `Return`). You should see the Node.js version printed (e.g., `v18.18.0`).
    *   Type `npm -v` (and press `Enter` or `Return`). You should see the npm version printed (e.g., `9.8.1`).
    *   (Tip: If a command isn't recognized, try closing your terminal window and opening a brand new one.)

### 4.2. Install Frontend Dependencies

Now that you have Node.js and npm, you can install all the specific libraries and packages Project Harvey's frontend needs.

1.  **Open a Terminal (or use VSCode's built-in terminal)**.
2.  **Navigate to the Project Harvey Folder**:
    *   Your terminal needs to be "inside" the main Project Harvey 1.0 folder that you downloaded (cloned) earlier. This is the folder that contains files like `package.json`, `src`, and `src-tauri`.
    *   Use the `cd` command (which means "change directory") to navigate there. For example, if your project is in `Documents/GitHub/project-harvey`:
        ```bash
        cd Documents/GitHub/project-harvey
        ```
        (Replace `Documents/GitHub/project-harvey` with the actual path on your computer. Press `Enter` or `Return` after typing the command.)
    *   (Tip: If you're unsure of the exact path to type, you can often drag the project folder directly from your File Explorer or Finder window into the terminal window, and it will paste the correct path for you. You'll still need to type `cd` before it and press `Enter`.)
    *   (Tip: If you have the project open in VSCode, you can open an integrated terminal by going to `Terminal > New Terminal` in the VSCode menu. It will usually start in the correct project folder automatically.)
3.  **Run `npm install`**:
    *   Once your terminal is focused on the **root directory** of the Project Harvey 1.0 project (the main folder containing `package.json`), type the following command and press `Enter` or `Return`:
        ```bash
        npm install
        ```
    *   **What this does**: This command reads the `package.json` file, which lists all the frontend libraries (or "packages") that Project Harvey 1.0 needs to function. `npm` then downloads these packages from the internet and installs them into a folder called `node_modules` within your project directory. Importantly, this step also installs the project-specific version of the Tauri CLI (`@tauri-apps/cli`), as mentioned in Part 3.3. This ensures you're using the correct CLI version for this project without needing a separate global installation.
    *   This process will print a lot of lines in the terminal as it downloads files. It might take a few minutes. Don't worry about 'WARN' (warning) messages; look for any major 'ERROR' messages at the very end. If there are no major errors, it's complete. A successful installation usually ends with messages about packages added or audited.

After this, the frontend part of Project Harvey 1.0, including the necessary local tools for building and running it, should be all set up! This project uses SvelteKit with JavaScript for its user interface.

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
    *   Make sure your terminal is in the root directory of Project Harvey 1.0 (the main folder containing `package.json`, `src`, and `src-tauri`, e.g., `cd path/to/your/project-harvey-folder`). Press `Enter` or `Return` after typing the `cd` command.
3.  **Run the Development Command**:
    *   Type the following command and press `Enter` or `Return`:
        ```bash
        npm run tauri dev
        ```
    *   **Why `npm run tauri dev`?** We use this command because our project manages the Tauri CLI tool as a development dependency through `npm` (Node Package Manager). The `npm run` part tells `npm` to execute the `tauri dev` command using the specific version of the CLI tools that the project expects, ensuring consistency.
    *   **What happens now?**
        *   This command tells Tauri to build both the frontend (SvelteKit) and the backend code (Rust) and then launch the application in "development mode."
        *   You'll see a lot of output in your terminal. The very first time you run this, it might take several minutes because Rust needs to compile all the backend code (you'll see messages about "compiling crates" – "crates" are what Rust calls its packages or libraries). Subsequent builds are usually much faster.
        *   If everything is set up correctly, after the build process finishes, the Project Harvey 1.0 application window should automatically open on your screen! You'll also see that your terminal window remains active, showing logs from the running application. Don't close this terminal window, or the application will close too.

    *   **Troubleshooting Tips**:
        *   **Correct Folder**: Make sure your terminal is definitely in the main Project Harvey 1.0 folder before running the command.
        *   **Backend Errors**: If you see errors mentioning "Rust," "cargo," or "Tauri" specifically, double-check that you completed all steps in "Part 3: Setting Up the Backend" correctly. The Tauri system prerequisites (Step 3.2) are especially common culprits if missed. (Tip: Try closing and reopening your terminal before running the command again if you just installed prerequisites.)
        *   **Frontend Errors**: If errors seem related to "npm," "Node.js," or specific JavaScript packages, review "Part 4" to ensure Node.js is installed and `npm install` completed without major errors.

### 5.2. Creating a Production Build (Optional)

If you want to create a standalone application package (like an `.exe` file on Windows, an `.app` file on macOS, or an executable file on Linux that you could share with others who don't have all these development tools), you can create a "production build."

1.  **Open a Terminal**.
2.  **Navigate to the Project Harvey Folder** (the main root directory, same as for the `dev` command). Press `Enter` or `Return` after your `cd` command.
3.  **Run the Build Command**:
    *   Type the following command and press `Enter` or `Return`:
        ```bash
        npm run tauri build
        ```
    *   Similar to the `dev` command, this uses the project-specific Tauri CLI via `npm`.
    *   This process will also take some time. Once it's finished, it will usually tell you where the built application files are located and what the file names are. This is typically in a subfolder within `src-tauri/target/release/` (for example, `src-tauri/target/release/bundle/msi/` for a Windows installer, or `src-tauri/target/release/bundle/dmg/` for a macOS disk image). The exact path will be shown in the terminal output. You can then navigate to this folder using your computer's file explorer to find the packaged application.

You've now successfully run Project Harvey 1.0! If you used the `dev` command, you can close the application window. To stop the development server that's running in your terminal, you can usually press `Ctrl+C` (hold down the `Ctrl` key and press `C`) in the terminal window.

---
## Part 6: How to Request a Feature or Report a Bug

Your feedback is valuable for making Project Harvey better! If you have an idea for a new feature, or if you encounter a problem (a "bug"), here’s how you can let the developers know through GitHub.

### 6.1. Understanding GitHub Issues

GitHub Issues is a tracking system built into every GitHub repository. It's used to track tasks, enhancements, and bugs for projects.

*   **Where to find it**:
    1.  Go to the Project Harvey 1.0 repository page on GitHub in your web browser (this is the same URL you used for cloning: `https://github.com/Ethnomethodology/harvey`).
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
        *   **Project Harvey 1.0 Version (if known)**: (If you're running from the code you just set up, you can mention it's the 'development version from [current date]' or if you know the specific branch or commit, that's even better, but not essential for a first report.)
        *   **Screenshots (if helpful)**: You can often drag and drop or attach images directly into the comment box on GitHub to upload them. A picture can often explain a problem much faster!

    *   **For Feature Requests, try to include**:
        *   **What problem does this feature solve?** Or, what new capability would it enable for users?
        *   **How would you ideally like it to work?** Describe your vision for the feature.
        *   **Are there any alternatives you've considered or seen in other software?** (Optional)

5.  **Use Markdown for Formatting**: You can use simple text formatting called Markdown (like `## Headings`, `* bullet points`, `` `code` `` for code snippets or commands, `**bold text**`) to make your issue easier to read. GitHub provides a formatting toolbar above the text area as well. (Tip: GitHub has a quick guide if you click the 'M' icon or 'Styling with Markdown is supported' link usually visible near the comment box.)
6.  **Submit the Issue**: Once you're satisfied with your report or request, click the "Submit new issue" button.

A project maintainer will review your issue and respond when they can. Your contributions, whether bug reports or feature ideas, are highly appreciated and help make Project Harvey 1.0 better for everyone!

Thank you for following this setup guide!
