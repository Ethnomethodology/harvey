import { test, expect } from '@playwright/test';
import path from 'path';
import fs from 'fs';

test.describe('Harvey App E2E Simulation (Desktop UI)', () => {
  // Generate a timestamp for this test run
  const timestamp = new Date().toISOString().replace(/[:.]/g, '-').slice(0, 19);
  const baseScreenshotsDir = 'e2e-tests/screenshots';
  const screenshotsDir = `${baseScreenshotsDir}/${timestamp}`;

  test.setTimeout(90000); // 5 steps * 5s = 25s + overhead + navigation
  test.beforeEach(async ({ page }) => {
    // 1. Mock the Tauri Environment
    // This allows the app logic to run as if it were in the native window
    await page.addInitScript(() => {
      window.__TAURI_INTERNALS__ = {
        invoke: async (cmd, args) => {
          console.log(`[Tauri Mock] Invoke command: ${cmd}`, args);
          
          // Mock Dialogs
          if (cmd === 'plugin:dialog|save') return 'temp/e2e-project';
          if (cmd === 'plugin:dialog|ask') return true;
          if (cmd === 'plugin:dialog|message') return;
          
          // Mock Path Plugin (Tauri v2)
          if (cmd === 'plugin:path|basename') return args.path.split(/[\\/]/).pop();
          if (cmd === 'plugin:path|dirname') return args.path.split(/[\\/]/).slice(0, -1).join('/') || '/';
          if (cmd === 'plugin:path|home_dir') return '/Users/mockuser';

          // Mock Project Creation/Opening
          if (cmd === 'create_project') return 'temp/e2e-project/harvey.xml';
          if (cmd === 'open_project') return;
          if (cmd === 'import_project') return { name: 'E2E-Project', path: 'temp/e2e-project/harvey.xml' };
          if (cmd === 'load_recent_projects') return [];
          
          // Mock Project View Data
          if (cmd === 'load_project_data') {
            return {
              id: 'e2e-uuid',
              name: 'E2E-Project',
              base_directory: 'temp/e2e-project',
              files: [],
              groups: [],
              tags: []
            };
          }
          
          if (cmd === 'check_config_status') return { 
              python_libraries_installed: true,
              hf_token_present: true,
              transcription_models_downloaded: true,
              diarization_model_downloaded: true,
              translation_models_downloaded: true
          };
          if (cmd === 'set_menu_context') return;

          return null;
        },
        metadata: {
            os: 'macos',
            arch: 'aarch64'
        }
      };
      
      // Traditional Tauri invoke fallback
      window.__TAURI_INVOKE__ = window.__TAURI_INTERNALS__.invoke;
      
      // Mock Event Listener
      window.__TAURI_INTERNALS__.listen = async (event, callback) => {
          console.log(`[Tauri Mock] Listening for event: ${event}`);
          return () => {}; // Unlisten
      };
    });
  });

  test('launch app, create project, navigate tabs, and close', async ({ page }) => {
    // Ensure screenshots directory exists
    if (!fs.existsSync(screenshotsDir)) {
      fs.mkdirSync(screenshotsDir, { recursive: true });
    }

    // 1. Launch the app (Vite dev server)
    await page.goto('http://localhost:1420');

    // Wait for the app to be ready (Heading "Harvey")
    const welcomeHeader = page.locator('h1', { hasText: 'Harvey' });
    await expect(welcomeHeader).toBeVisible({ timeout: 15000 });
    await page.waitForTimeout(5000);
    await page.screenshot({ path: `${screenshotsDir}/1-app-launched.png` });

    // 2. Create Project
    const createBtn = page.getByRole('button', { name: 'Create Project' });
    await createBtn.click();
    
    // The mocked dialog should immediately handle the save. 
    // Now wait for the transition to the Project View (Data tab)
    const dataTab = page.locator('button[aria-label="Data"]');
    await expect(dataTab).toBeVisible({ timeout: 10000 });
    await page.waitForTimeout(5000);
    await page.screenshot({ path: `${screenshotsDir}/2-project-view-data.png` });

    // 3. Transcription Tab
    const transcriptionTab = page.locator('button[aria-label="Transcription"]');
    await transcriptionTab.click();
    await page.waitForTimeout(5000);
    await page.screenshot({ path: `${screenshotsDir}/3-transcription-tab.png` });

    // 4. Tags Tab
    const tagsTab = page.locator('button[aria-label="Tags"]');
    await tagsTab.click();
    await page.waitForTimeout(5000);
    // 5. Return to Projects (Close/Back)
    // In the simulation, we navigate back to the root to "close" the project
    await page.goto('http://localhost:1420/'); 
    await expect(welcomeHeader).toBeVisible();
    await page.waitForTimeout(5000);
    await page.screenshot({ path: `${screenshotsDir}/5-returned-home.png` });
  });
});
