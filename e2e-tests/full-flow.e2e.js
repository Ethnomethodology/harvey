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

        // Delete the dummy project folder if it exists from a previous run.
        // Do NOT recreate it here — the Rust backend creates the directory itself
        // when initialising a new project. Pre-creating it triggers E_DIR_EXISTS.
        const dummyProjectDir = path.join(__dirname, '..', 'e2e-tests', 'dummy-project-folder');
        if (fs.existsSync(dummyProjectDir)) {
            fs.rmSync(dummyProjectDir, { recursive: true, force: true });
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
        // Since the window controls (close, minimize, maximize) are native OS elements,
        // WebdriverIO cannot interact with them via CSS selectors.
        // We simulate the native close event by calling the exposed handler directly.
        await browser.execute(() => {
            if (typeof window.__E2E_CLOSE_PROJECT__ === 'function') {
                window.__E2E_CLOSE_PROJECT__();
            } else {
                console.error("E2E close project hook not found on window object.");
            }
        });

        await browser.pause(5000);
        await browser.saveScreenshot(path.join(screenshotsDir, '5-returned-home.png'));
    });
});
