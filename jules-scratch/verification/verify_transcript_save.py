import asyncio
from playwright.async_api import async_playwright
import os

async def main():
    async with async_playwright() as p:
        try:
            # Use a persistent context to grant file system permissions
            # This is a common approach for Tauri apps
            context = await p.chromium.launch_persistent_context(
                "/tmp/playwright_user_data",
                headless=True,
                args=["--no-sandbox"],  # Recommended for CI environments
            )
            page = await context.new_page()

            # The app runs on localhost:1420 by default in dev mode
            await page.goto("http://localhost:1420", timeout=60000)

            # Wait for the welcome screen to load
            await page.wait_for_selector('h1:has-text("Harvey")', timeout=30000)

            # 1. Create a new project
            await page.get_by_role("button", name="Create Project").click()

            # The project creation dialog is handled by the backend, so we can't
            # interact with it directly. We'll assume the user creates a project
            # in the `jules-scratch/test-project` directory.
            # We will create the project structure manually to simulate this.
            project_path = os.path.abspath("jules-scratch/test-project")
            if not os.path.exists(project_path):
                os.makedirs(project_path)

            # Create a dummy project file
            with open(os.path.join(project_path, "harvey.json"), "w") as f:
                f.write('{"name": "Test Project"}')

            # Reload the page to see the new project
            await page.reload()
            await page.wait_for_selector('h2:has-text("Recent Projects")', timeout=30000)

            # Open the newly created project
            await page.get_by_text("Test Project").click()

            # 2. Navigate to the "Data" tab
            await page.get_by_role("button", name="Data").click()

            # 3. Import the transcript
            # Click the "Import" button for the "Imported Transcripts" category
            await page.locator('button[title="Import Imported Transcripts"]').click()

            # The file chooser is handled by the backend, so we need to
            # use a different approach than page.on("filechooser").
            # We'll assume the app is running with the necessary permissions
            # and that the file dialog will open. We can't interact with it
            # directly, but we can check for the result of the import.

            # Since we can't interact with the file dialog, we will have to assume
            # that the user would select the dummy_transcript.txt file.
            # This is a limitation of testing Tauri apps with Playwright.
            # We will proceed with the assumption that the file is imported.

            # Let's create a dummy file in the project to simulate the import
            transcript_content = "This is a dummy transcript file.\\nIt has some text to highlight.\\nAnd a second line of text."
            await page.evaluate(f'''
                window.__TAURI__.fs.writeFile({{
                    path: "{project_path}/data/imported_transcripts/dummy_transcript.txt",
                    contents: "{transcript_content}"
                }})
            ''')

            # Refresh the file list
            await page.get_by_role("button", name="Data").click()
            await page.get_by_role("button", name="Data").click()

            # 4. Open the imported transcript
            await page.get_by_text("dummy_transcript.txt").click()

            # 5. Add a highlight
            await page.locator(".lexical-editor").select_text()
            await page.get_by_title("Highlight selection yellow").click()

            # 6. Add a tag
            await page.get_by_placeholder("Add tag...").click()
            await page.get_by_placeholder("Add tag...").fill("test-tag")
            await page.get_by_text("+ Create new tag \"test-tag\"").click()

            # 7. Navigate away
            await page.get_by_text("Project").click()

            # 8. Navigate back
            await page.get_by_text("dummy_transcript.txt").click()

            # 9. Take a screenshot
            await page.screenshot(path="jules-scratch/verification/verification.png")

            # 10. Assert that the highlight and tag are present
            highlight_is_visible = await page.locator('span[style="background-color: yellow;"]').is_visible()
            tag_is_visible = await page.get_by_text("test-tag").is_visible()

            if not highlight_is_visible:
                raise Exception("Highlight not found after reload.")

            if not tag_is_visible:
                raise Exception("Tag not found after reload.")

            print("Verification successful!")

        except Exception as e:
            print(f"An error occurred: {e}")
            await page.screenshot(path="jules-scratch/verification/error.png")
        finally:
            await context.close()

if __name__ == "__main__":
    asyncio.run(main())
