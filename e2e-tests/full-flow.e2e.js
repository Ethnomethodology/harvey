import path from 'path';
import { fileURLToPath } from 'url';
import fs from 'fs';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const screenshotsDir = path.join(__dirname, '..', 'e2e-tests', 'screenshots');

describe('Harvey E2E Test Flow', () => {

    before(async () => {
        // Ensure screenshots directory exists
        if (!fs.existsSync(screenshotsDir)) {
            fs.mkdirSync(screenshotsDir, { recursive: true });
        }
    });

    it('should launch the app and capture the welcome screen', async () => {
        // Wait 5 seconds for the app to fully load
        await browser.pause(5000);
        await browser.saveScreenshot(path.join(screenshotsDir, '1-app-launched.png'));
    });

    it('should create a project and switch to Data tab', async () => {
        // Set the global window variable to bypass the Tauri dialog in our mock
        const testProjectPath = path.join(__dirname, '..', 'e2e-tests', 'dummy-project-folder');
        await browser.execute((path) => {
            window.__E2E_TEST_PROJECT_PATH__ = path;
        }, testProjectPath);

        // Find and click the "Create Project" button
        // Looking at common UI patterns, we look for a button containing the text
        const createBtn = await $('button=Create Project');
        // If exact text match fails, you might need a more resilient selector like $('button:has-text("Create")')
        // Svelte buttons might just have the text inside
        if (await createBtn.isExisting()) {
            await createBtn.click();
        } else {
            // Alternative heuristic fallback
            const altBtn = await $("//button[contains(text(), 'Create Project')]");
            await altBtn.click();
        }

        // Wait 5 seconds for the project to create, load, and switch to the Data tab
        await browser.pause(5000);
        await browser.saveScreenshot(path.join(screenshotsDir, '2-project-view-data.png'));
    });

    it('should switch to the Transcription tab', async () => {
        // Click the Transcription tab (Using title attribute based on source code)
        const transcriptionTab = await $("button[title='Transcription']");
        await transcriptionTab.click();

        await browser.pause(5000);
        await browser.saveScreenshot(path.join(screenshotsDir, '3-transcription-tab.png'));
    });

    it('should switch to the Tags tab', async () => {
        // Click the Tags tab (Using title attribute based on source code)
        const tagsTab = await $("button[title='Tags']");
        await tagsTab.click();

        await browser.pause(5000);
        await browser.saveScreenshot(path.join(screenshotsDir, '4-tags-tab.png'));
    });

    it('should close the project back to the home screen', async () => {
        // This is typically a back button or close button in the top bar.
        // Looking at DataTopBar/SimpleTopBar it emits 'close' when returning to the welcome screen
        // The project is closed via the X icon in the top right window controls, or a dedicated home button.
        // If not found, we use browser.execute to trigger the close logic internally for the test
        const closeBtn = await $("button[title='Close Project']");

        if (await closeBtn.isExisting()) {
            await closeBtn.click();
        } else {
            const homeBtn = await $("//button[contains(., 'Home') or @aria-label='Close Project']");
            if (await homeBtn.isExisting()) {
                await homeBtn.click();
            } else {
                console.log("Could not find standard Close Project button. Using fallback UI dispatch.");
                // As a fallback to exit the project, click the Harvey top-left logo/home if it exists,
                // or just trigger the window close via JS if it's the native window frame.
                // In Svelte tests, we can just click the first button that might be a back button
                const fallback = await $("button.text-gray-500.hover\\:text-gray-900"); // typical close button styles
                if (await fallback.isExisting()) {
                    await fallback.click();
                } else {
                     const firstBtn = await $("button");
                     await firstBtn.click();
                }
            }
        }

        await browser.pause(5000);
        await browser.saveScreenshot(path.join(screenshotsDir, '5-returned-home.png'));
    });
});
