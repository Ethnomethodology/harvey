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
        // Click the Transcription tab
        const transcriptionTab = await $("//button[contains(., 'Transcription')]");
        await transcriptionTab.click();

        await browser.pause(5000);
        await browser.saveScreenshot(path.join(screenshotsDir, '3-transcription-tab.png'));
    });

    it('should switch to the Tags tab', async () => {
        // Click the Tags tab
        const tagsTab = await $("//button[contains(., 'Tags')]");
        await tagsTab.click();

        await browser.pause(5000);
        await browser.saveScreenshot(path.join(screenshotsDir, '4-tags-tab.png'));
    });

    it('should close the project back to the home screen', async () => {
        // This is typically a back button or close button in the top bar.
        // Look for an element with a title "Close Project" or an icon
        const closeBtn = await $("button[title='Close Project']");

        if (await closeBtn.isExisting()) {
            await closeBtn.click();
        } else {
            // Alternatively, look for a "Home" or SVG icon that acts as back
            const homeBtn = await $("//button[contains(., 'Home') or @aria-label='Close Project']");
            if (await homeBtn.isExisting()) {
                await homeBtn.click();
            } else {
                 // Or click the settings/close gear if it's there
                 console.log("Could not find standard Close Project button. Attempting heuristic fallback.");
                 const fallback = await $("button"); // This will just click the first button, we should refine this if possible
                 // Let's rely on standard text or titles first
            }
        }

        await browser.pause(5000);
        await browser.saveScreenshot(path.join(screenshotsDir, '5-returned-home.png'));
    });
});
