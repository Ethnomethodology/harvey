import path from 'path';
import { fileURLToPath } from 'url';
import fs from 'fs';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const logsDir = path.join(__dirname, '..', 'e2e-tests', 'logs');

describe('Harvey E2E Test Flow', () => {

    async function saveLogs(stageName) {
        const logs = await browser.execute(() => {
            const currentLogs = window.__E2E_LOGS__ || [];
            // Clear logs after reading so we only get new ones for the next stage?
            // Actually, the user asked for logs in those 5 stages. I'll take all logs so far.
            return currentLogs;
        });
        
        fs.writeFileSync(path.join(logsDir, `${stageName}.log`), logs.join('\n'));
    }

    before(async () => {
        // Ensure logs directory exists
        if (!fs.existsSync(logsDir)) {
            fs.mkdirSync(logsDir, { recursive: true });
        }

        // Delete the dummy project folder if it exists from a previous run.
        const dummyProjectDir = path.join(__dirname, '..', 'e2e-tests', 'dummy-project-folder');
        if (fs.existsSync(dummyProjectDir)) {
            fs.rmSync(dummyProjectDir, { recursive: true, force: true });
        }
    });

    it('should launch the app and inject log capture', async () => {
        // Wait 5 seconds for the app to fully load
        await browser.pause(5000);
        
        // Inject log capture script as early as possible
        await browser.execute(() => {
            window.__E2E_LOGS__ = [];
            const originalConsole = {
                log: console.log,
                error: console.error,
                warn: console.warn,
                info: console.info
            };
            const capture = (type, args) => {
                const timestamp = new Date().toISOString();
                const message = args.map(arg => {
                    try {
                        return typeof arg === 'object' ? JSON.stringify(arg) : String(arg);
                    } catch (e) {
                        return String(arg);
                    }
                }).join(' ');
                window.__E2E_LOGS__.push(`[${timestamp}] [${type.toUpperCase()}] ${message}`);
                originalConsole[type].apply(console, args);
            };
            console.log = (...args) => capture('log', args);
            console.error = (...args) => capture('error', args);
            console.warn = (...args) => capture('warn', args);
            console.info = (...args) => capture('info', args);
            console.log("E2E Log Capture Initialized");
        });

        await saveLogs('1-app-launched');
    });

    it('should create a project and switch to Data tab', async () => {
        // Set the global window variable to bypass the Tauri dialog in our mock
        const testProjectPath = path.join(__dirname, '..', 'e2e-tests', 'dummy-project-folder');
        await browser.execute((path) => {
            window.__E2E_TEST_PROJECT_PATH__ = path;
        }, testProjectPath);

        // Find and click the "Create Project" button
        const createBtn = await $('button=Create Project');
        if (await createBtn.isExisting()) {
            await createBtn.click();
        } else {
            const altBtn = await $("//button[contains(text(), 'Create Project')]");
            await altBtn.click();
        }

        // Wait 5 seconds for the project to create, load, and switch to the Data tab
        await browser.pause(5000);
        await saveLogs('2-project-view-data');
    });

    it('should switch to the Transcription tab', async () => {
        // Click the Transcription tab (Using title attribute based on source code)
        const transcriptionTab = await $("button[title='Transcription']");
        await transcriptionTab.click();

        await browser.pause(5000);
        await saveLogs('3-transcription-tab');
    });

    it('should switch to the Tags tab', async () => {
        // Click the Tags tab (Using title attribute based on source code)
        const tagsTab = await $("button[title='Tags']");
        await tagsTab.click();

        await browser.pause(5000);
        await saveLogs('4-tags-tab');
    });

    it('should close the project back to the home screen', async () => {
        // We simulate the native close event by calling the exposed handler directly.
        await browser.execute(() => {
            if (typeof window.__E2E_CLOSE_PROJECT__ === 'function') {
                window.__E2E_CLOSE_PROJECT__();
            } else {
                console.error("E2E close project hook not found on window object.");
            }
        });

        await browser.pause(5000);
        await saveLogs('5-returned-home');
    });
});
