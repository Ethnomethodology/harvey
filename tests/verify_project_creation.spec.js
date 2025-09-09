import { test, expect, webkit } from '@playwright/test';
import path from 'path';
import fs from 'fs';

test.beforeAll(() => {
  fs.mkdirSync(path.resolve('./temp/test-project'), { recursive: true });
});

test('Create project, open it, and screenshot each tab', async () => {
  test.setTimeout(60000);
  const browser = await webkit.launch({
    executablePath: path.resolve('./src-tauri/target/release/harvey'),
  });
  const context = await browser.newContext();
  const appWindow = await context.newPage();

  appWindow.on('console', console.log);

  // Wait for the app to be ready
  await appWindow.waitForSelector('text="Welcome to Harvey"');

  // Click the "Create New Project" button
  await appWindow.click('button:has-text("Create New Project")');

  // Wait for the create project modal to appear
  await appWindow.waitForSelector('h2:has-text("Create New Project")');

  // Set project name
  await appWindow.fill('input[placeholder="Untitled Project"]', 'test-project');

  // Click the "Select Project Directory" button and handle the file dialog
  const [fileChooser] = await Promise.all([
    appWindow.waitForEvent('filechooser'),
    appWindow.click('button:has-text("Select Project Directory")'),
  ]);
  await fileChooser.setFiles(path.resolve('./temp/test-project'));


  // Click the "Save" button
  await appWindow.click('button:has-text("Save")');

  // Wait for the project view to load by looking for the Data tab
  await appWindow.waitForSelector('button[aria-label="Data"]');

  // Define tabs to screenshot
  const tabs = ["Data", "Transcriptions", "Documents", "Images", "Tables", "Tags"];

  for (const tab of tabs) {
    console.log(`Clicking on ${tab} tab and taking screenshot...`);
    await appWindow.click(`button[aria-label="${tab}"]`);
    // Give some time for the tab content to render
    await appWindow.waitForTimeout(1000);
    await appWindow.screenshot({ path: `playwright-report/tab-${tab.toLowerCase()}-screenshot.png` });
  }

  // Close the app
  await browser.close();
});