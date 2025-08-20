import re
import os
from playwright.sync_api import sync_playwright, Page, expect

def run(playwright):
    browser = playwright.chromium.launch(headless=True)
    context = browser.new_context()
    page = context.new_page()

    try:
        # Define project path
        project_name = "test-project"
        base_dir = os.getcwd()
        project_dir = os.path.join(base_dir, "jules-scratch", "verification", "test-project")
        project_xml_path = os.path.join(project_dir, f"{project_name}.harvey.xml")

        # Go to the welcome page
        page.goto("http://localhost:4173/", timeout=60000)

        # We can't use `openProjectWindow` directly from actions.js because it creates a new window,
        # which is hard to manage in this script.
        # Instead, we will navigate directly to the project view URL.
        # The URL format is `/projectview?xmlPath=<encoded_path>`
        encoded_path = project_xml_path.replace("/", "%2F")
        project_view_url = f"http://localhost:4173/projectview?xmlPath={encoded_path}"

        page.goto(project_view_url, timeout=60000)

        # Wait for the project view to load by looking for a known element
        # Let's look for the data view container
        expect(page.locator(".data-view-container")).to_be_visible(timeout=30000)

        page.screenshot(path="jules-scratch/verification/02_project_view.png")

        # Now, let's try to import a transcript
        # We need to find the button to import a transcript.
        # Looking at the codebase, it's likely in `DataTopBar.svelte`
        # The button has the text "Import"
        page.get_by_role("button", name="Import").click()

        # The import button opens a modal. We need to interact with the modal.
        # The modal has a title "Import Transcript From..."
        expect(page.get_by_text("Import Transcript From...")).to_be_visible()

        # In the modal, there are different import options. We'll choose "Plain Text (.txt)"
        page.get_by_role("button", name="Plain Text (.txt)").click()

        # This will open a file dialog. We need to handle it.
        # We can't handle the dialog directly, but we can see what function is called.
        # The "Plain Text (.txt)" button in `ImportTranscriptSourceModal.svelte` calls `onSelectImportType('text')`.
        # This calls `handleImport('text')`.
        # In `DataTopBar.svelte`, `handleImport` calls `invoke('import_transcript', ...)`
        # This is what we need to call.

        # We will need to provide the path to the transcript file.
        transcript_path = os.path.join(base_dir, "jules-scratch", "verification", "transcript.txt")

        # We can use page.evaluate to call the invoke command.
        # We need to pass the file path to the command.
        page.evaluate(f"window.__TAURI_INTERNALS__.invoke('import_transcript', {{ path: '{transcript_path}', assetType: 'transcript' }})")

        # After importing, the view should change to the transcript editor.
        # Let's wait for the editor to appear.
        expect(page.locator(".transcript-editor-panel")).to_be_visible(timeout=30000)
        page.screenshot(path="jules-scratch/verification/03_transcript_editor.png")

        # Now, let's add a highlight.
        # We need to select some text and then click the highlight button.
        # The editor is a Lexical editor, so we can interact with it.
        editor = page.locator(".lexical-editor")
        editor.focus()
        page.keyboard.press("Control+A") # Select all text

        # Now click the highlight button.
        # From the code, the highlight button is in a floating toolbar.
        # The toolbar appears when text is selected.
        # Let's just apply a highlight programmatically to avoid dealing with the floating toolbar.
        page.evaluate("""
            const editor = document.querySelector('.lexical-editor').__lexicalEditor;
            editor.update(() => {
                const selection = $getSelection();
                if ($isRangeSelection(selection)) {
                    $patchStyleText(selection, { 'background-color': 'yellow' });
                }
            });
        """)
        page.screenshot(path="jules-scratch/verification/04_with_highlight.png")

        # Now, let's add a tag.
        # The tag component is `TagMultiSelect.svelte`.
        # It has an input field.
        tag_input = page.locator(".tag-multiselect input")
        tag_input.type("test-tag")
        page.keyboard.press("Enter")

        # Check that the tag was added
        expect(page.get_by_text("test-tag")).to_be_visible()
        page.screenshot(path="jules-scratch/verification/05_with_tag.png")

        # Now, reload the page
        page.reload()

        # Wait for the editor to load again
        expect(page.locator(".transcript-editor-panel")).to_be_visible(timeout=30000)

        # Check if the highlight is still there.
        # This is tricky. We can't easily check the background color.
        # Instead, we can check the editor state.
        # The highlights are stored in the project store.
        # Let's check the screenshot.
        page.screenshot(path="jules-scratch/verification/06_after_reload.png")

        # And check if the tag is still there.
        expect(page.get_by_text("test-tag")).to_be_visible()

    except Exception as e:
        print(f"An error occurred: {e}")
        page.screenshot(path="jules-scratch/verification/error.png")
        # Save page source for debugging
        with open("jules-scratch/verification/page_source.html", "w") as f:
            f.write(page.content())

    finally:
        browser.close()

with sync_playwright() as playwright:
    run(playwright)
