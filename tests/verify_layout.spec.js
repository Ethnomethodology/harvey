import { test, expect, _electron as electron } from '@playwright/test';
import path from 'path';

test('Create project and capture tab screenshots', async () => {
  const electronApp = await electron.launch({
    args: ['./src-tauri/target/release/harvey'],
  });

  const appWindow = await electronApp.firstWindow();

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
  await fileChooser.setFiles(path.resolve('./jules-scratch/test-project'));


  // Click the "Save" button
  await appWindow.click('button:has-text("Save")');

  // Wait for the project view to load by looking for the Data tab
  await appWindow.waitForSelector('button[aria-label="Data"]');

  // Take a screenshot of the Data tab
  await appWindow.screenshot({ path: 'jules-scratch/screenshot-data-before.png' });

  // Click the "Transcriptions" tab
  await appWindow.click('button[aria-label="Transcriptions"]');
  // Add a small delay to ensure tab content loads
  await appWindow.waitForTimeout(500);
  await appWindow.screenshot({ path: 'jules-scratch/screenshot-transcriptions-before.png' });

  // Click the "Tags" tab
  await appWindow.click('button[aria-label="Tags"]');
  // Add a small delay to ensure tab content loads
  await appWindow.waitForTimeout(500);
  await appWindow.screenshot({ path: 'jules-scratch/screenshot-tags-before.png' });

  // Close the app
  await electronApp.close();
});
