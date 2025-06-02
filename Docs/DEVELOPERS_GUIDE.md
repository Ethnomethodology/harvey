# Project Harvey 1.0 - Developer Contribution Guide

Welcome, future contributor! This guide is specially designed for novice developers and those new to programming who are excited to contribute to Project Harvey, especially if you plan to use AI-assisted coding tools like Google's "Jules."

**The Purpose of This Guide:**

*   To help you understand the basic tools and concepts needed for contributing.
*   To provide a step-by-step workflow for making your first contribution using AI assistance.
*   To highlight important open-source practices that ensure your contributions are helpful and can be easily integrated into the project.

We believe that with the right guidance and tools, anyone can make valuable contributions!

**Prerequisites:**

Before you dive into this guide, please make sure you have successfully set up Project Harvey on your computer by following our **`SETUP_GUIDE.md`**. This setup guide covers installing essential tools like Git, GitHub Desktop, Rust, and Node.js, and also explains how to download (clone) the project's code onto your machine.
*   **(You can find a link to it here: [./SETUP_GUIDE.md](./SETUP_GUIDE.md) - it's in the same `Docs` folder as this guide.)**

Once your setup is complete, you're ready to learn how to contribute!

---
## Part 2: Understanding Key Tools & Concepts

Before we dive into making changes, let's clarify a few important tools and ideas that you'll encounter. Don't worry if these are new to you – we'll keep it simple!

### 2.1. Git & Version Control

*   **What is Git?**
    Imagine Git as a very smart "undo" and history tracking system for an entire software project. It keeps a record of every change made to the code, who made each change, and when. It's also designed to allow many people to work on the same project at the same time without (too much) chaos. This whole process of tracking and managing changes is called "version control." You've already installed tools that use Git in the `SETUP_GUIDE.md`.

*   **What is a Repository (often called a "Repo")?**
    A repository is basically a project's main folder, containing all its files and the complete history of changes that Git is tracking. When you followed the `SETUP_GUIDE.md` to "clone" Project Harvey, you downloaded a copy of its repository from GitHub to your computer.

*   **Branches: Working on a Separate Copy**
    *   **What is a branch?** Think of a tree. The main, strong trunk is the primary, stable version of the project's code (this is often called the `main` branch, though sometimes it might be named `master`). When you want to add a new feature or fix a bug, you (or your AI assistant) will create a "branch." This is like a new limb growing out from the trunk. It's a separate, independent copy of the code where you can make your changes safely without affecting the stable `main` version until you're ready.
    *   **Why use branches?**
        *   **Safety**: Keeps the `main` branch clean and working. If you make a mistake on your branch, it doesn't break the main project for everyone else.
        *   **Collaboration**: Multiple people can work on different features on different branches at the same time.
        *   **Review**: When you're done with your changes on a branch, others can review them before they get merged into the `main` branch.
    *   **(Note: Your AI coding assistant, like Jules, will usually handle the technical step of creating a new branch for you when you start working on a new task based on a GitHub Issue.)**

### 2.2. GitHub Desktop: Your Visual Git Helper

While Git itself is a command-line tool (meaning you type commands into a terminal), GitHub Desktop provides a user-friendly graphical interface (with buttons and menus) to perform most common Git tasks. This is why we recommended it in the `SETUP_GUIDE.md`.

*   **What it helps with**:
    *   Seeing what files you've changed.
    *   "Committing" your changes (this means saving a snapshot of your work to Git's history, like creating a named save point).
    *   "Pushing" your changes (this means sending your saved commits from your computer up to the main GitHub repository, making them visible to others or your AI).
    *   "Fetching" and "Pulling" changes (this means getting the latest updates from the GitHub repository to your computer, for example, if your AI assistant made changes on a branch or if other contributors updated the `main` branch).
    *   Switching between different branches.

    We'll cover these actions in the workflow part!

### 2.3. AI Coding Assistants (e.g., Jules)

AI coding assistants, like Google's Jules (which you can access by visiting its website, typically [https://jules.google.com/](https://jules.google.com/) - check their official site for the current URL), are powerful tools that can help you write, understand, and modify code.

*   **How they can help Project Harvey contributors**:
    *   **Generating Code**: You can describe a feature you want to add or a bug you want to fix (based on a GitHub Issue), and the AI can suggest or even write the necessary code.
    *   **Explaining Code**: If you don't understand a piece of Project Harvey's code, you can ask the AI to explain it.
    *   **Planning Changes**: The AI can help create a plan (a list of steps) to implement a feature or fix a bug.
    *   **Debugging**: If something is not working, the AI might help you find the problem.

*   **Important Principles when using AI Assistants**:
    *   **Clear Instructions are Key**: The AI is not a mind-reader. You need to give it clear, specific instructions (this is called "prompting"). The better your prompt, the better the AI's help.
    *   **Review the AI's Work Critically**: Always carefully review any code, explanations, or plans the AI suggests. It's a tool to assist you, not to replace your critical thinking. Make sure the changes make logical sense and actually solve the problem described in the GitHub Issue.
    *   **You Are Responsible**: Ultimately, you are the one submitting the contribution. You are responsible for its quality and correctness, even if an AI helped write parts of it. This means testing the changes thoroughly is essential (see Part 3.4).

---
## Part 3: Your First Contribution Workflow (AI-Assisted)

This section will guide you through making a typical contribution to Project Harvey using an AI coding assistant like Jules.

### Step 3.1: Always Start with a GitHub Issue!

Before you (or an AI) write any code, you need a starting point and an agreement with the project maintainers.

*   **Why this is crucial**:
    *   **Avoid Wasted Effort**: If you work on something the maintainers don't think fits the project, or if someone else is already working on it, your hard work might not be accepted.
    *   **Get Clarity**: Discussing the bug or feature first helps everyone understand the problem and the proposed solution. You might get valuable advice or direction.
    *   **Tracking**: Issues help track what's being worked on.
*   **How to do it**:
    *   Go to Project Harvey's GitHub page: [https://github.com/Ethnomethodology/harvey](https://github.com/Ethnomethodology/harvey)
    *   Click on the "Issues" tab.
    *   **Search first!** See if a similar issue already exists.
    *   If not, click "New issue" to create one.
    *   **Refer to our `SETUP_GUIDE.md` (Part 6)** for detailed advice on how to write a good bug report or feature request. A clear and well-described issue is the best possible starting point for both you and your AI assistant.
    *   Wait for a project maintainer (one of the core developers) to respond to your issue. They might ask clarifying questions, offer suggestions, or give you a "green light" to proceed. This communication is key!

### Step 3.2: Setting Up and Interacting with Your AI Coding Assistant (Jules Example)

Once you have a GitHub Issue that a maintainer has acknowledged and agreed is ready for work (let's say it's Issue #123 for our example), you can engage your AI assistant.

*   **Accessing the AI**:
    *   For this example, we'll use "Jules." You can typically access it by going to its website, like [https://jules.google.com/](https://jules.google.com/).
    *   **(Important Note: Always make sure you are using AI tools like Jules responsibly. Be aware of their terms of service, data privacy policies, and any limitations they might have. Do not share any secret information or private data with the AI.)**

*   **Read the AI's Own Documentation First!**
    *   Before you start, it's very important to read the specific instructions, tutorials, and best practice guides provided by the AI tool itself (e.g., on the jules.google.com website if you're using Jules). These official guides will teach you the most effective and safe ways to use that particular AI. This guide provides general steps, but the AI's own documentation is your primary resource for using the tool itself.

*   **Key Interaction Steps with an AI Assistant like Jules**:

    1.  **Giving the Task (Effective "Prompting")**:
        *   "Prompting" is how you give instructions to the AI. Start a new conversation or session.
        *   Be very clear, specific, and detailed about what you want the AI to do.
        *   **Crucially, provide context!** Don't assume the AI knows anything about Project Harvey. You need to tell it:
            *   The project name: "Project Harvey"
            *   The GitHub Issue URL or at least the number: "I'm working on Issue #123 from the Project Harvey GitHub (Ethnomethodology/harvey)." You can even paste the description from the issue.
            *   The problem or goal: Clearly describe the bug or the feature.
            *   Any specific requirements or constraints mentioned in the GitHub Issue.
        *   **Example Prompt for Harvey**:
            ```
            Hello Jules, I need your help with Project Harvey (GitHub: Ethnomethodology/harvey).
            I'm working on Issue #123, which is: "The application crashes when trying to import very large (>1GB) MP3 files."

            The goal is to prevent this crash. Ideally, the application should:
            1. Check the file size *before* trying to process the entire MP3.
            2. If the file is too large (e.g., over 1GB, but maybe you can suggest a reasonable limit or find if one is already defined), it should *not* attempt to import it.
            3. Instead, it should show a user-friendly error message like, "This MP3 file is too large to import. Please try a smaller file."
            4. The application should remain stable and not crash.

            Can you help me identify the relevant files in the Project Harvey codebase and suggest the specific code changes needed to implement this? I'd also like you to create a new Git branch for these changes.
            ```

    2.  **Reviewing the AI's Plan and Asking Questions**:
        *   The AI (like Jules) will likely analyze your request and propose a plan. This might include a list of files it thinks need to be changed and a summary of the proposed modifications.
        *   **Read this plan very carefully.** This is a critical step.
            *   Does the AI's interpretation of the problem match yours and what's in the GitHub Issue?
            *   Do the suggested changes make logical sense to you as a way to solve the problem?
            *   Even if you don't understand all the code details, do the files it wants to change seem like the right general areas (e.g., if it's an import problem, are the files related to importing)?
        *   **If you're unsure about any part of the plan, ask the AI to explain it!** For example: "Can you explain why you chose to modify [specific file name]?" or "What is the purpose of the [specific function name] you're suggesting?" Don't just blindly approve its plan. Use it as a learning opportunity.

    3.  **Iterating and Providing Feedback (Working *with* the AI)**:
        *   Sometimes the AI might ask you questions to clarify the task or if it has multiple options. Answer them as clearly as you can.
        *   If the AI's initial plan or code doesn't seem quite right, or if it missed something, give it feedback. For example: "That looks like a good start, but the error message should also suggest checking the file's integrity, not just its size." Or, "You've addressed the MP3 issue, but does this also affect WAV file imports?"
        *   You might go back and forth with the AI a few times to refine the plan or the code. This is a normal part of the process.

    4.  **Authorizing the AI to Make Code Changes (If applicable for your AI tool)**:
        *   Some AI tools, like Jules, might be able to directly make changes to your code repository on GitHub if you grant them permission. Other AI tools might just provide you with code snippets that you then have to copy and paste into the files yourself using VSCode.
        *   **If the AI can make changes directly**: Once you are satisfied with the plan and understand what the AI is proposing, you might give it your approval to proceed. The AI will then typically write or modify the code on a new branch.
        *   **If the AI provides code snippets**: You will need to carefully identify the correct files and locations in your project (using VSCode) and paste the code in. This is where understanding the AI's plan is vital.
        *   **Always ask the AI for the name of the new branch** it created (or that you should create if pasting code manually). Branch names are often descriptive, like `fix-large-mp3-import-crash` or `feature-new-button-settings`.

### Step 3.3: Getting the AI's Changes to Your Computer (Using GitHub Desktop)

Now that the AI has made changes on a new branch in the GitHub repository, you need to bring those changes to your local copy of the project on your computer.

1.  **Open GitHub Desktop**.
2.  **Ensure you are on the Project Harvey repository** (it should show "Current repository: harvey" or similar at the top).
3.  **Fetch Origin**:
    *   At the top of GitHub Desktop, click the "Fetch origin" button (it might look like a cloud with a down arrow, or a refresh-like button). Sometimes, it might say "Fetch" and then change to "Pull origin" automatically if there are new changes on your current branch.
    *   **What this does**: "Fetching" updates GitHub Desktop's knowledge of all branches and commits (saved changes) that are on the main GitHub repository (often called the "origin" or "remote"). It doesn't usually change your actual project files yet; it just gets the latest information *about* what's on GitHub.
    *   You should see a progress indicator. After it's done, it might say "Fetch complete" or immediately show if there are changes to pull.

4.  **Switch to the AI's Branch**:
    *   Click on the "Current Branch" button in GitHub Desktop (it usually shows the name of the branch you were on last, perhaps `main`).
    *   A dropdown list of branches will appear. Look for the branch name the AI gave you or that you created (e.g., `fix-large-mp3-import-crash`).
        *   It might be listed under "Recent Branches" if the AI just created it and you've fetched.
        *   If you don't see it, you can type part of the branch name into the "Filter branches" search box at the top of the dropdown.
    *   Click on the AI's branch name in the list to select it.
    *   **What this does**: This tells your local Git repository (your copy of the project) that you now want to view and work on this new branch. GitHub Desktop will update your project files on your computer to match the state of this branch as it exists locally.

5.  **Pull Changes to Get the Latest from GitHub**:
    *   After switching to the branch, GitHub Desktop might show a "Pull origin" button with a number next to it (this number indicates how many new commits are on GitHub for this branch that you don't have yet). Or it might have already pulled if it was a brand new branch for your local machine.
    *   If you see "Pull origin," click it.
    *   **What this does**: "Pulling" actually downloads the committed changes from that specific branch on GitHub to your local copy of that same branch. This ensures your project files are updated with what the AI (or anyone else) pushed to that branch on GitHub.

    Now, the code in your Project Harvey folder on your computer should match the changes made by the AI on that specific branch!

### Step 3.4: Testing the AI's Changes Locally

This is a **CRITICALLY IMPORTANT** step. Never assume the AI's changes are perfect or complete. You must test!

1.  **Run the Application in Development Mode**:
    *   Open your terminal (or VSCode's integrated terminal, which is often easier).
    *   Make sure your terminal is "in" the main Project Harvey root folder (the one with `package.json`).
    *   Run the command: `npm run tauri dev` (and press `Enter` or `Return`).
    *   Wait for the application to build and launch.

2.  **Test the Specific Feature or Bug Fix**:
    *   Go through the steps that would trigger the feature or bug you were trying to address.
    *   In our example (Issue #123), try to import a very large MP3 file.
        *   Does it correctly show the error message instead of crashing?
        *   Try importing a small MP3 file. Does that still work correctly?
        *   Try importing other supported audio files. Do they still work?

3.  **Test Related Functionality (Regression Testing)**:
    *   Think about what other parts of the application *might* have been affected by the changes, even indirectly.
    *   For example, if the change involved import logic, test importing other types of files (videos, documents if applicable). Test saving and loading projects.
    *   The goal is to ensure the AI's changes didn't accidentally break something else. Click around and use the application as a normal user would in areas related to the change.

### Step 3.5: Handling Issues or Needed Revisions from AI Changes

It's common for AI-generated code to need some adjustments or for the first attempt not to be perfect.

*   **If the changes didn't work as expected, or if you found a new problem**:
    1.  **Gather Information**:
        *   Note down exactly what happened.
        *   Copy any error messages from the terminal or the application console (if you know how to access it – your AI might be able to guide you on this too).
        *   List the steps you took to reproduce the problem.
    2.  **Report Back to your AI Assistant (Jules)**:
        *   Go back to your conversation with the AI.
        *   Clearly explain the problem you encountered during testing. Provide the information you gathered.
        *   **Example Feedback to Jules**:
            ```
            Hi Jules, I tested the changes for Issue #123 on the 'fix-large-mp3-import-crash' branch.
            The good news is the app no longer crashes with large MP3s!
            However, the error message "This MP3 file is too large to import. Please try a smaller file." doesn't appear. Instead, nothing happens, the import dialog just closes.
            I expected the error message to pop up.
            Can you look into why the message isn't showing and adjust the code for the 'fix-large-mp3-import-crash' branch?
            ```
    3.  **Let the AI Make Revisions (or Make Them Yourself)**:
        *   The AI will likely propose new changes or a revised plan. Discuss it again if needed.
        *   If the AI can make changes directly to the branch, you might authorize it again.
        *   Alternatively, if the AI suggests specific code snippets, you might decide to manually apply them yourself using VSCode. This is a good way to get more familiar with the code. If you do this, refer to Part 4.1 for committing your manual changes.
    4.  **Get the Latest Revisions (if AI made them on GitHub)**:
        *   If the AI pushed new changes to the branch on GitHub, go back to GitHub Desktop.
        *   Make sure you are still on the correct branch (e.g., `fix-large-mp3-import-crash`).
        *   Click "Fetch origin" again.
        *   Then, click "Pull origin" to download the latest revisions.
    5.  **Re-Test**: Repeat Step 3.4 (Testing the AI's Changes Locally) thoroughly.

    *   Continue this cycle of testing, providing feedback (to the AI or by making your own fixes), committing, pushing (if you made local changes), and pulling until the feature or bug fix works correctly and doesn't introduce new problems.

---
## Part 4: Basic Git Operations with GitHub Desktop (and a few commands)

While your AI assistant might handle many Git operations (like creating branches and committing its main code changes directly to GitHub), you'll often need to make small manual edits, or you'll want to understand a bit more about what GitHub Desktop is doing.

### 4.1. Committing Your Own Small Changes with GitHub Desktop

Let's say after your AI (like Jules) made its changes, you noticed a small typo in a comment or an error message it wrote. You can fix this directly in VSCode.

1.  **Make Your Edit**: Open the relevant file in VSCode (which should be showing the AI's branch if you followed Part 3.3) and make your small correction. Save the file.
2.  **View Changes in GitHub Desktop**:
    *   Open GitHub Desktop.
    *   On the left side, under the "Changes" tab, you will see a list of files that you've modified. The file you just edited should be there.
    *   Click on the file name to see the exact changes you made (lines added will be green, lines removed will be red).
3.  **Stage Your Changes**:
    *   By default, all changes in modified files are usually "staged" (meaning selected to be included in the next save point or "commit"). You'll see a checkmark next to the file. If it's unchecked for some reason, check it to include its changes.
4.  **Write a Commit Message**:
    *   At the bottom left of GitHub Desktop, there's a "Summary" field (required) and a larger "Description" field (optional).
    *   For a small fix, a clear summary is often enough (e.g., "Fix typo in large MP3 import error message").
    *   The description is optional but useful if you need to explain your manual changes in more detail.
5.  **Commit to Your Branch**:
    *   Click the "Commit to [branch-name]" button (where `[branch-name]` is the name of the branch you are currently on, e.g., `fix-large-mp3-import-crash`).
    *   **What this does**: A "commit" saves a snapshot of your selected (staged) changes to the Git history *on your local computer* for this specific branch. It's like saving your game at a checkpoint, giving it a name (the commit message).

### 4.2. Pushing Changes to GitHub

After you've committed changes locally on your computer, you need to "push" them to the remote GitHub repository. This updates the branch on GitHub with your new commits, makes your changes available to others (like project maintainers or your AI if it reads from the repo), and ensures they are backed up.

1.  **Look for the "Push origin" Button**:
    *   After committing, the main button in GitHub Desktop often changes to "Push origin". It might also show a number next to it indicating how many local commits are ready to be pushed (sent to GitHub).
    *   (If your AI assistant has already pushed its latest changes to the branch on GitHub, and you haven't made any *new local* commits after that, you might not need to do this step.)
2.  **Click "Push origin"**:
    *   This uploads your committed changes from your local computer to the corresponding branch on GitHub.

### 4.3. Keeping Your Branch Updated with `main` (More Advanced, Optional for First Timers)

Sometimes, while you're working on your feature or bug-fix branch, the main `main` branch of Project Harvey might get updated by other contributors. It's a good practice (though can be a bit more advanced for your very first contribution) to occasionally bring these new changes from the `main` branch into your working branch. This helps prevent big differences and potential "merge conflicts" (where changes clash) later on when you create your Pull Request.

*   **How to do it (Simplified with GitHub Desktop)**:
    1.  First, make sure your own branch (`fix-large-mp3-import-crash`) has all its changes committed and pushed.
    2.  Switch to the `main` branch: In GitHub Desktop, click "Current Branch," select `main`, and click "Fetch origin" then "Pull origin" to make sure your local `main` is fully up-to-date.
    3.  Switch back to your working branch (e.g., `fix-large-mp3-import-crash`).
    4.  In the top menu of GitHub Desktop, go to `Branch > Update from main` or `Branch > Merge into Current Branch...` (and select `main`). GitHub Desktop will guide you.
    *   If there are "merge conflicts" (where changes in `main` are in the exact same lines of code as your changes, and Git doesn't know which version to keep), GitHub Desktop will try to help you resolve them. This can be tricky for a beginner. If you encounter this, it's a good time to pause and ask for help from the project maintainers or a more experienced developer. Your AI assistant might also offer guidance on resolving specific conflicts if you can describe them to it.
*   **(Note: For your first few contributions, you probably won't need to do this step unless a project maintainer specifically asks you to. Don't worry about it for now if it seems too complex.)**

### 4.4. Common Git Commands (Just for Your Awareness)

You'll primarily use GitHub Desktop for these operations. However, it's useful to know that GitHub Desktop is running more fundamental Git commands behind the scenes. You don't need to type these if you're using GitHub Desktop, but seeing them might help you understand forum posts or AI explanations better in the future:

*   `git add [filename]`: This command stages a specific file for a commit (similar to checking the box next to a file in GitHub Desktop's "Changes" tab).
*   `git commit -m "Your commit message"`: This saves your staged changes to your local Git history along with a descriptive message.
*   `git push origin [branch-name]`: This uploads your local commits from your specified branch to the remote repository on GitHub (often just `git push` is enough if the branch is set up to track the remote).
*   `git pull origin [branch-name]`: This fetches changes from the specified branch on the remote repository and attempts to merge them into your current local branch.
*   `git branch [new-branch-name]`: This creates a new branch on your local machine.
*   `git checkout [branch-name]`: This switches your current working files to match the specified branch. (Like selecting a branch from the "Current Branch" dropdown in GitHub Desktop).
*   `git merge [other-branch-name]`: This combines changes from the `other-branch-name` into your currently active branch.

Don't worry about memorizing these commands now, or ever, if you prefer using GitHub Desktop! It's just to show you a little of what's happening underneath.

---
## Part 5: Submitting Your Contribution (Creating a Pull Request)

Once you and your AI assistant have completed the changes on your branch, and you've tested them thoroughly (Step 3.4 and 3.5), it's time to propose that your work be merged into the main Project Harvey codebase. In GitHub, this is done by creating a "Pull Request" (often abbreviated as "PR"). A Pull Request is a formal way to tell others about changes you've pushed to a branch in a repository on GitHub and request that they be reviewed and incorporated.

### Step 5.1: Ensure Your Branch is Pushed to GitHub

1.  **Open GitHub Desktop**.
2.  Make sure you are on your working branch (e.g., `fix-large-mp3-import-crash`).
3.  If you have any uncommitted local changes, commit them (see Part 4.1).
4.  If the "Push origin" button is active (meaning you have local commits not yet on GitHub), click it to push your latest changes. Your branch on GitHub should now be up-to-date with your local copy.

### Step 5.2: Open a Pull Request on GitHub

1.  **Go to the Project Harvey GitHub Repository**:
    *   Open your web browser and navigate to [https://github.com/Ethnomethodology/harvey](https://github.com/Ethnomethodology/harvey).
2.  **GitHub Often Prompts You**:
    *   If you've recently pushed a new branch to the repository, GitHub often shows a yellow notification bar near the top of the repository page with your branch name and a button that says "Compare & pull request". This is the easiest way! Click that button.
3.  **Alternatively, Start Manually**:
    *   If you don't see that prompt, click on the "Pull requests" tab near the top of the repository page.
    *   On the Pull requests page, click the green "New pull request" button.
4.  **Choose the Branches to Compare**:
    *   **Base Branch**: This is the branch you want to merge *into*. For Project Harvey, this will almost always be `main`. Ensure the "base:" dropdown is set to `main`.
    *   **Compare Branch**: This is *your* branch that has the new feature or bug fix (e.g., `fix-large-mp3-import-crash`). Select your branch from the "compare:" dropdown.
    *   GitHub will show you a comparison of the changes between your branch and `main`.

### Step 5.3: Write a Clear and Detailed Pull Request Description

This is a **very important** part of your contribution. A good PR description helps maintainers understand your changes, why you made them, and how to review them.

*   **Title**:
    *   Make it clear and concise, summarizing the change. Often, it can be similar to your branch name or the issue it addresses.
    *   Example: `Fix: Prevent crash when importing oversized MP3 files (Closes #123)`

*   **Description (the main text box)**:
    *   **Link to the GitHub Issue**:
        *   **This is usually mandatory!** Start by stating which issue your PR addresses. Use special keywords like "Closes #123", "Fixes #456", or "Resolves #789" (replace the number with the actual issue number). This automatically links the PR to the issue, and can even auto-close the issue when the PR is merged.
        *   Example: `This pull request closes Issue #123.`
    *   **What was done?**
        *   Briefly explain the changes you (and your AI assistant) made. You don't need to explain every line of code, but give a summary of the approach.
        *   Example: "Implemented a file size check before the import process for MP3 files. If a file exceeds 1GB, an error message is now displayed to the user, and the import is aborted, preventing a potential crash."
    *   **How does the new feature work / How was the bug fixed?**
        *   Provide context for the changes.
    *   **How did you test your changes?**
        *   Describe the testing you performed. This gives maintainers confidence in your work.
        *   Example: "Tested by attempting to import an MP3 file of 1.5GB (correctly showed error message), an MP3 of 500MB (imported successfully), and confirmed that WAV file imports are unaffected. Also checked that project saving/loading still works."
    *   **Mention AI Assistance (Transparency is good!)**:
        *   It's good practice to mention if you used an AI coding assistant.
        *   Example: "An AI coding assistant (Google's Jules) was used to help generate the initial code for the file size check and the error handling logic. The code and plan were reviewed and tested thoroughly."
    *   **Screenshots or GIFs (if helpful)**:
        *   If your change affects the user interface (e.g., a new button, an error message), adding a screenshot or a short animated GIF can be very helpful. You can drag and drop images directly into the description box on GitHub.

### Step 5.4: Create and Monitor the Pull Request

1.  **Review Your PR Details**: Read through your title and description one last time.
2.  **Click "Create pull request"**.
3.  **Discussion and Review**:
    *   Project maintainers will now be notified of your PR. They will review your code and description.
    *   They might:
        *   Approve it and "merge" it into the `main` branch (Congratulations!).
        *   Ask questions or request changes.
        *   Provide feedback or suggestions.
    *   **Respond to Feedback**: Check back on your PR page for comments. Engage in the discussion politely and address any concerns or requests. You might need to make further changes on your branch (with your AI or manually), commit them, and push them again. The PR will update automatically with your new commits.

Congratulations! You've learned how to submit a contribution. It's a collaborative process!

---
## Part 6: Important Reminders for AI-Assisted Development

Using AI coding assistants like Jules can be a fantastic way to learn and contribute, but it's important to keep a few things in mind:

1.  **You Are Ultimately Responsible for Your Contributions.**
    *   AI tools are assistants. They can make mistakes or produce code that isn't perfect or doesn't fully consider the broader context of the project.
    *   **Always review and test the AI's suggestions thoroughly.** Do not blindly accept and submit code generated by an AI without understanding it and verifying its correctness and impact.
    *   The quality and integrity of the contribution are your responsibility.

2.  **Use AI as a Learning Opportunity.**
    *   When an AI generates code or a plan, try to understand *why* it suggested that approach.
    *   If you don't understand something, ask the AI to explain it! For example: "Jules, can you explain why you chose to use [specific function/method] here?" or "What does this block of code do?"
    *   This is a great way to learn programming concepts and how Project Harvey is built.

3.  **Be Mindful of Security, Privacy, and Licensing.**
    *   **Do not paste sensitive information** (like personal API keys, passwords, or private data) into prompts for public AI tools.
    *   Be aware of the terms of service of the AI tool you are using.
    *   Ensure that any code suggested by the AI is compatible with Project Harvey's open-source license (usually, AI tools are trained on publicly available code, but it's good to be mindful). If the AI references specific libraries, ensure they are compatible.

4.  **Follow Project-Specific Contribution Guidelines.**
    *   This guide provides general advice. However, Project Harvey might have its own specific `CONTRIBUTING.md` file or rules mentioned in the `README.md` or on its GitHub page. Always look for these and follow them.
    *   These might include coding style preferences, specific testing requirements, or how to format commit messages and Pull Request titles.

5.  **Communicate Clearly.**
    *   When interacting with the AI, be as clear and specific in your prompts as possible.
    *   When interacting with human project maintainers (in GitHub Issues and Pull Requests), also communicate clearly, politely, and provide all necessary information.

6.  **Start Small.**
    *   For your first few contributions, especially as a novice developer, pick small, well-defined bugs or simple feature enhancements. This will help you get familiar with the process without being overwhelmed.

AI-assisted development is an evolving field. By being a responsible and curious user of these tools, you can make significant and valuable contributions to Project Harvey.

Happy coding, and thank you for your interest in contributing!
