import path from 'path';
import fs from 'fs';

describe('Project Creation and Tab Screenshots', () => {
  before(() => {
    fs.mkdirSync(path.resolve('./temp/test-project'), { recursive: true });
  });

  it('should create a new project and take screenshots of all tabs', async () => {
    // Wait for the app to be ready
    const welcomeText = await $('h1');
    await expect(welcomeText).toHaveText('Welcome to Harvey');

    // Click the "Create New Project" button
    const createProjectButton = await $('button=Create New Project');
    await createProjectButton.click();

    // Wait for the create project modal to appear
    const createProjectModal = await $('h2=Create New Project');
    await createProjectModal.waitForExist({ timeout: 5000 });

    // Set project name
    const projectNameInput = await $('input[placeholder="Untitled Project"]');
    await projectNameInput.setValue('test-project');

    // For the file dialog, we can't interact with it directly.
    // In a real scenario, this would need a more robust solution,
    // like using tauri's APIs to set the project directory.
    // For this test, we will just click the button and hope for the best.
    const selectDirButton = await $('button=Select Project Directory');
    await selectDirButton.click();

    // We need to handle the file chooser here.
    // Since we can't, we will just wait for a bit and assume it worked.
    await new Promise(resolve => setTimeout(resolve, 2000));


    // Click the "Save" button
    const saveButton = await $('button=Save');
    await saveButton.click();

    // Wait for the project view to load by looking for the Data tab
    const dataTab = await $('button[aria-label="Data"]');
    await dataTab.waitForExist({ timeout: 10000 });

    // Define tabs to screenshot
    const tabs = ["Data", "Transcription", "Documents", "Images", "Tables", "Tags"];

    for (const tab of tabs) {
      console.log(`Clicking on ${tab} tab and taking screenshot...`);
      const tabButton = await $(`button[aria-label="${tab}"]`);
      await tabButton.click();
      await new Promise(resolve => setTimeout(resolve, 1000));
      await browser.saveScreenshot(`./playwright-report/tab-${tab.toLowerCase()}-screenshot.png`);
    }
  });
});
