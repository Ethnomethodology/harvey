import { expect } from '@wdio/globals'

describe('Project Creation and Tab Screenshots', () => {
	it('should create a new project and take screenshots of all tabs', async () => {
		// Wait for the main window to be visible
		await browser.waitUntil(async () => {
			const handles = await browser.getWindowHandles();
			return handles.length > 0;
		}, {
			timeout: 20000,
			timeoutMsg: 'Main window did not appear in time'
		});

		// 1. Take a screenshot of the initial welcome screen
		await browser.saveScreenshot('./screenshots/1_welcome-screen.png');

		// 2. Create a project directly by invoking the Tauri command
		console.log("Attempting to create project via Tauri command...");
		const projectXmlPath = await browser.executeAsync(async (done) => {
			const { invoke } = window.__TAURI__.core;
			try {
				// Use a temporary directory for the test project
				const projectPath = await invoke('create_project', {
					name: 'E2E Test Project',
					parentLocation: '/tmp', // Using /tmp as a reliable temporary location
					overwrite: true // Overwrite to ensure clean state for each run
				});
				console.log('E2E project created at:', projectPath);
				done(projectPath);
			} catch (error) {
				console.error('E2E project creation via invoke failed:', error);
				done(null);
			}
		});

		expect(projectXmlPath).to.not.be.null;
		console.log(`Project created successfully, XML path: ${projectXmlPath}`);

		// Add a pause to allow the project view to load after creation.
		await browser.pause(5000);

		// 3. Take a screenshot of the default "Data" tab.
		await browser.saveScreenshot('./screenshots/2_project-view-data-tab.png');
		console.log("Successfully took screenshot of Data tab.");

		// 4. Navigate to the "Transcription" tab and take a screenshot
		const transcriptionTab = await $('[title="Transcription"]');
		await transcriptionTab.waitForExist({ timeout: 10000 });
		await transcriptionTab.click();
		await browser.pause(1000); // Wait for tab content to render
		await browser.saveScreenshot('./screenshots/3_project-view-transcription-tab.png');
		console.log("Successfully took screenshot of Transcription tab.");

		// 5. Navigate to the "Tags" tab and take a screenshot
		const tagsTab = await $('[title="Tags"]');
		await tagsTab.waitForExist({ timeout: 10000 });
		await tagsTab.click();
		await browser.pause(1000); // Wait for tab content to render
		await browser.saveScreenshot('./screenshots/4_project-view-tags-tab.png');
		console.log("Successfully took screenshot of Tags tab.");
	});
});
