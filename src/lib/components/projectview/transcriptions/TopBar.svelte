<!-- src/lib/components/projectview/transcriptions/TopBar.svelte -->
<script>
	// --- Svelte/Store Imports ---
	import { createEventDispatcher, onMount } from 'svelte';
	import { get } from 'svelte/store';
	import { project } from '$lib/stores/projectStore.js'; // For project-level state like isLoading, files, isTranscribing
	import { transcriptStore, setSelectedModel, setSelectedLanguage, updateSpeakerConfig, selectMedia, setTranslateToEnglish } from '$lib/stores/transcriptStore.js';
	import { themePreference, cycleThemePreference } from '$lib/stores/themeStore.js';

	// --- Service Imports ---
	import { requestTranscription } from '$lib/services/projectService.js';
	import { getDownloadedModels, getCloudConfig, exportTranscript } from '$lib/services/configureActions.js';

	// --- Tauri Imports ---
	import { message } from '@tauri-apps/plugin-dialog';

	// --- Child Component Imports ---
	
	import SpeakersModal from '../modals/SpeakersModal.svelte';
	import ExportModal from '../modals/ExportModal.svelte';
	import LayoutSettingsModal from '../modals/LayoutSettingsModal.svelte';
	import { activeLayout, leftPanelVisible } from '$lib/stores/layoutStore.js';
	import { languageOptions, getCloudModelLabel } from '$lib/constants/transcriptionOptions.js';

	// --- Local state ---
	const dispatch = createEventDispatcher();
	let downloadedModelsList = [];
	let cloudConfig = null;
	let isLoadingModels = true;
	let isManageModalOpen = false;
	let isSpeakersModalOpen = false;
	let isExportModalOpen = false;
	let isLayoutSettingsModalOpen = false; // Added
	let transcriptionMode = 'automatic';
	// Variable to hold transcript path for export modal
	let transcriptPathForExport = '';

	function handleAddBlankTranscript() {
		console.log('Add Blank Transcript clicked');
		// TODO: implement blank transcript creation
	}

	// --- Load Configuration ---
	async function loadConfiguration() {
        isLoadingModels = true;
        cloudConfig = null;
        try {
            const [localModelsResult, cloudConfigResult] = await Promise.allSettled([ getDownloadedModels(), getCloudConfig() ]);
            if (localModelsResult.status === 'fulfilled') {
                downloadedModelsList = localModelsResult.value;
                console.log("TopBar: Loaded local models:", downloadedModelsList);
            } else {
                console.error("TopBar: Failed to load local models", localModelsResult.reason);
                downloadedModelsList = []; // Ensure it's an array on error
            }
            if (cloudConfigResult.status === 'fulfilled') {
                cloudConfig = cloudConfigResult.value;
                console.log("TopBar: Loaded cloud config:", cloudConfig);
            } else {
                console.error("TopBar: Failed to load cloud config", cloudConfigResult.reason);
                cloudConfig = null; // Ensure it's null on error
            }
        } catch (e) {
            console.error("TopBar: Error during configuration loading:", e);
            downloadedModelsList = [];
            cloudConfig = null;
        } finally {
            isLoadingModels = false;
            validateSelectedModel();

            // --- ADDED: Set Default Model and Language ---
            const currentTranscriptState = get(transcriptStore); // Get current state non-reactively

            // Set default model if none selected
            if (!currentTranscriptState.selectedModelName) {
                let defaultModel = downloadedModelsList[0]?.name; // Try first local model
                if (!defaultModel && cloudConfig?.consent && cloudConfig?.model) {
                    defaultModel = cloudConfig.model; // Try configured cloud model
                }
                if (defaultModel) {
                    console.log(`[TopBar] No model selected, setting default: ${defaultModel}`);
                    setSelectedModel(defaultModel);
                } else {
                    console.log('[TopBar] No model selected and no default available.');
                }
            } else {
                 console.log(`[TopBar] Model already selected: ${currentTranscriptState.selectedModelName}`);
            }

            // Set default language if none selected
            if (!currentTranscriptState.selectedLanguage) {
                 console.log('[TopBar] No language selected, setting default: auto');
                 setSelectedLanguage('auto'); // Default to Auto Detect
            } else {
                 console.log(`[TopBar] Language already selected: ${currentTranscriptState.selectedLanguage}`);
            }
            // --- END ADDED ---
        }
    }


	// --- Validate Selected Model ---
	function validateSelectedModel() { const currentSelectedModel = $transcriptStore.selectedModelName; if (!currentSelectedModel) return; let isModelValid = false; if (downloadedModelsList.some(m => m.name === currentSelectedModel)) { isModelValid = true; } else if ( cloudConfig?.consent && cloudConfig.api_key && cloudConfig.model && cloudConfig.model === currentSelectedModel ) { isModelValid = true; } if (!isModelValid) { console.warn(`TopBar: Previously selected model "${currentSelectedModel}" no longer valid. Resetting selection.`); setSelectedModel(null); } }

	// --- Lifecycle ---
	onMount(async () => { await loadConfiguration(); });

	// --- Event Handlers ---
	function handleTranscribeClick() {
		console.log('TopBar: Transcribe icon clicked');
		if (!$transcriptStore.selectedMediaFile?.path) {
			message("Please select a media file first.", { title: "No Media Selected", type: "warning" });
			return;
		}
		
		if (!$transcriptStore.selectedLanguage) {
			message("Please select the audio language first.", { title: "No Language Selected", type: "warning" });
			return;
		}
		requestTranscription(); // This service function will now internally get state from transcriptStore or be passed it
	}

	function openExportModal() {
		if ($transcriptStore.segments?.length > 0 && $transcriptStore.currentTranscriptPath) {
			console.log('Export icon clicked, opening export modal');
			transcriptPathForExport = $transcriptStore.currentTranscriptPath;
			isExportModalOpen = true;
		} else {
			console.log('Export icon clicked but no transcript loaded to export.');
			message('No transcript data loaded to export.', { title: "Cannot Export", type: "info" });
		}
	}
	async function handleExportConfirm(event) {
		const { filePath, format, layoutChoice } = event.detail;
		console.log('TopBar: Export modal confirmed. Exporting:', { filePath, format, layoutChoice });
		const segmentsToExport = $transcriptStore.segments;
		if (!segmentsToExport || segmentsToExport.length === 0) {
			console.error("TopBar: Cannot export, no segments available in store.");
			message("No transcript data available to export.", { title: "Export Failed", type: "error" });
			return;
		}
		try {
			await exportTranscript(filePath, format, segmentsToExport, transcriptPathForExport, layoutChoice);
			console.log(`TopBar: Export to ${filePath} (${format}, Layout: ${layoutChoice || 'N/A'}) successful.`);
			message(`Transcript successfully exported to ${filePath}`, { title: "Export Successful", type: "info" });
		} catch (error) {
			console.error(`TopBar: Export failed to ${filePath} (${format}, Layout: ${layoutChoice || 'N/A'}):`, error);
			message(`Failed to export transcript: ${error?.message || error}`, { title: "Export Failed", type: "error" });
		}
	}
	async function handleModelChange(event) { const selectedValue = event.target.value; if (selectedValue === '__manage__') { console.log('TopBar: Manage Models selected'); isManageModalOpen = true; event.target.value = $transcriptStore.selectedModelName || ""; } else { const newModelIdentifier = selectedValue === "" ? null : selectedValue;
 console.log('TopBar: Selected model identifier:', newModelIdentifier || 'None'); setSelectedModel(newModelIdentifier);
 const currentLang = $transcriptStore.selectedLanguage;
 const localModelInfo = downloadedModelsList.find(m => m.name === newModelIdentifier);
 if (localModelInfo && currentLang && currentLang !== 'en') {
 console.log(`TopBar: Local model changed ('${newModelIdentifier}') while non-English language ('${currentLang}') active.`);
 await showModelInfoDialog(localModelInfo);
 }
 }
 }
	async function handleLanguageChange(event) { const selectedValue = event.target.value;
 const newLanguage = selectedValue === "" ? null : selectedValue;
 console.log('TopBar: Selected language:', newLanguage || 'None');
 setSelectedLanguage(newLanguage);
 const currentModelIdentifier = $transcriptStore.selectedModelName;
 if (newLanguage && newLanguage !== 'en' && currentModelIdentifier) {
 const localModelInfo = downloadedModelsList.find(m => m.name === currentModelIdentifier);
 if (localModelInfo) {
 console.log(`TopBar: Non-English language ('${newLanguage}') selected while LOCAL model ('${currentModelIdentifier}') active.`);
 await showModelInfoDialog(localModelInfo);
 }
 }
 }
	async function showModelInfoDialog(modelInfo) { if (!modelInfo) return;
 let infoMessage = `Model Information: ${modelInfo.name}\n\n`;
 if (modelInfo.description && modelInfo.description.trim() !== '') {
 infoMessage += `${modelInfo.description}\n\n`;
 } else if (modelInfo.language && modelInfo.language.trim() !== '') {
 infoMessage += `Primary Language Focus: ${modelInfo.language}\n`;
 } else {
 infoMessage += "General purpose model.\n";
 }
 if (modelInfo.size && modelInfo.size.trim() !== '') {
 infoMessage += `Size: ${modelInfo.size}`;
 }
 if (modelInfo.language && modelInfo.language.toLowerCase().includes('multilingual')) {
 infoMessage += "\n\nNote: This is a multilingual model.";
 } else if (modelInfo.language && !modelInfo.language.toLowerCase().startsWith('en')) {
 infoMessage += `\n\nNote: This model is primarily optimized for ${modelInfo.language}.`;
 }
 infoMessage += `\n\nFor more details, refer to the source where the model was downloaded.`;
 await message(infoMessage, { title: `Model Info: ${modelInfo.name}`, type: 'info', okLabel: 'OK' });
 }
	async function handleManageModalClose() { console.log("TopBar: Manage Models modal closed. Refreshing ALL configuration..."); await loadConfiguration(); } // loadConfiguration itself will update transcriptStore via setSelectedModel if needed
	function openSpeakersModal() { isSpeakersModalOpen = true; }
	function handleSpeakersConfirm(event) { const { count, names, secondNames } = event.detail;
 console.log("TopBar: Confirmed speakers:", count, names, secondNames);
 updateSpeakerConfig(count, names, secondNames);
 }
	function handleMediaSelectionChange(event) {
		const selectedPath = event.target.value;
		if (!selectedPath) { return; }
		const currentDropdownList = mediaFilesForDropdown;
		const selectedFileEntry = currentDropdownList.find(f => f.path === selectedPath);

		if (selectedFileEntry) {
			console.log('[TopBar] Media selected via dropdown:', selectedFileEntry.name);
			selectMedia(selectedFileEntry);
		} else {
			console.warn('[TopBar] Could not find FileEntry in dropdown list for selected path:', selectedPath);
		}
	}

	// --- Helper computed values for binding ---
	$: modelSelectValue = $transcriptStore.selectedModelName ?? "";
	$: languageSelectValue = $transcriptStore.selectedLanguage ?? "";

	// Ensure translateToEnglish is false if language is switched to English
	$: if ($transcriptStore.selectedLanguage === 'en' && $transcriptStore.translateToEnglish) {
		setTranslateToEnglish(false);
	}

	// --- Reactive check for Transcribe button disable state ---
	$: isTranscribeDisabled = !($transcriptStore.selectedMediaFile?.path);


	$: isExportDisabled = !$transcriptStore.activeTranscript?.path || !$transcriptStore.segments || $transcriptStore.segments.length === 0 || $project.isTranscribing || $project.isLoading; // isTranscribing and isLoading can remain from projectStore

	$: mediaFilesForDropdown = (() => {
		const rootNodes = $project.files || []; // files list still comes from projectStore
		const mediaFiles = [];
		function findMediaFilesRecursive(nodes) {
			for (const node of nodes) {
				if (node.file_type === 'media' && !node.is_directory) {
					mediaFiles.push(node);
				}
				if (node.children && Array.isArray(node.children) && node.children.length > 0) {
					findMediaFilesRecursive(node.children);
				}
			}
		}
		findMediaFilesRecursive(rootNodes);
		mediaFiles.sort((a, b) => a.name.localeCompare(b.name));
		return mediaFiles;
	})();

	$: selectedMediaValue = $transcriptStore.selectedMediaFile?.path ?? "";

	// --- Theme Icons ---
	const SUN_ICON = `<svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4"><path stroke-linecap="round" stroke-linejoin="round" d="M12 3v2.25m6.364.386-1.591 1.591M21 12h-2.25m-.386 6.364-1.591-1.591M12 18.75V21m-4.773-4.227-1.591 1.591M5.25 12H3m4.227-4.773L5.636 5.636M15.75 12a3.75 3.75 0 1 1-7.5 0 3.75 3.75 0 0 1 7.5 0Z" /></svg>`;
	const MOON_ICON = `<svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4"><path stroke-linecap="round" stroke-linejoin="round" d="M21.752 15.002A9.72 9.72 0 0 1 18 15.75c-5.385 0-9.75-4.365-9.75-9.75 0-1.33.266-2.597.748-3.752A9.753 9.753 0 0 0 3 11.25C3 16.635 7.365 21 12.75 21a9.753 9.753 0 0 0 9.002-5.998Z" /></svg>`;
	const SYSTEM_ICON = `<svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4"><path stroke-linecap="round" stroke-linejoin="round" d="M9 17.25v1.007a3 3 0 0 1-.879 2.122L7.5 21h9l-.621-.621A3 3 0 0 1 15 18.257V17.25m6-12V15a2.25 2.25 0 0 1-2.25 2.25H5.25A2.25 2.25 0 0 1 3 15V5.25m18 0A2.25 2.25 0 0 0 18.75 3H5.25A2.25 2.25 0 0 0 3 5.25m18 0V12a2.25 2.25 0 0 1-2.25 2.25H5.25A2.25 2.25 0 0 1 3 12V5.25" /></svg>`;
	$: themeIconHtml = $themePreference === 'light' ? SUN_ICON
					 : $themePreference === 'dark' ? MOON_ICON
					 : SYSTEM_ICON;
	$: nextThemeName = $themePreference === 'light' ? 'Dark'
					 : $themePreference === 'dark' ? 'System'
					 : 'Light';
	$: themeTitle = `Switch to ${nextThemeName} Mode`;

	// --- Layout Button Icon ---
	const LAYOUT_ICON_SVG = `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-layout-wtf" viewBox="0 0 16 16"><path d="M5 1v8H1V1zM1 0a1 1 0 0 0-1 1v8a1 1 0 0 0 1 1h4a1 1 0 0 0 1-1V1a1 1 0 0 0-1-1zm13 2v5H9V2zM9 1a1 1 0 0 0-1 1v5a1 1 0 0 0 1 1h5a1 1 0 0 0 1-1V2a1 1 0 0 0-1-1zM5 13v2H3v-2zm-2-1a1 1 0 0 0-1 1v2a1 1 0 0 0 1 1h2a1 1 0 0 0 1-1v-2a1 1 0 0 0-1-1zm12-1v2H9v-2zm-6-1a1 1 0 0 0-1 1v2a1 1 0 0 0 1 1h6a1 1 0 0 0 1-1v-2a1 1 0 0 0-1-1z"/></svg>`;

	function openLayoutSettingsModal() {
		isLayoutSettingsModalOpen = true;
	}

	function handleLayoutSelected(event) {
		const newLayoutKey = event.detail;
		activeLayout.setLayout(newLayoutKey);
		// Modal closes itself on selection
	}

	function toggleLeftPanel() {
		leftPanelVisible.toggle();
	}

</script>

<!-- Top Bar Structure -->
<div
	class="flex items-center justify-between px-1 h-10 flex-shrink-0 bg-white dark:bg-surface-1 border-b border-gray-200 dark:border-dark-bg-tertiary"
	data-tauri-drag-region
>
	<!-- Left Controls: Toggle Panel, Media Select, Model Select, Language Select, Speakers, Transcribe -->
	<div class="flex items-center space-x-1.5">
        <div class="h-10 flex items-center justify-center flex-shrink-0">
            <button title="Import" aria-label="Import" class="ui-button-import hover-scale-effect ml-1 mr-1" on:click={(e) => dispatch('requestImport', e)}>
                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="size-6">
                    <path stroke-linecap="round" stroke-linejoin="round" d="M12 4.5v15m7.5-7.5h-15" />
                </svg>
            </button>
        </div>
		<!-- Toggle Left Panel Button -->
		<button
			class="ui-button-icon-no-border p-1.5 hover-scale-effect mr-2"
			title="Toggle File Explorer Panel"
			on:click={toggleLeftPanel}
		>
			<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-layout-sidebar" viewBox="0 0 16 16">
				<path d="M0 3a2 2 0 0 1 2-2h12a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2H2a2 2 0 0 1-2-2zm5-1v12h9a1 1 0 0 0 1-1V3a1 1 0 0 0-1-1zM4 2H2a1 1 0 0 0-1 1v10a1 1 0 0 0 1 1h2z"/>
			 </svg>
		</button>

		<!-- Media Selection Dropdown -->
		<select
			class="ui-select text-sm text-gray-700 dark:text-gray-200 flex-shrink-0 w-40"
			on:change="{handleMediaSelectionChange}"
			bind:value="{selectedMediaValue}"
			disabled="{$project.isLoading || mediaFilesForDropdown.length === 0}"
			title="{mediaFilesForDropdown.length > 0 ? 'Select Media File' : ($project.isLoading ? 'Loading project...' : 'No media files found')}"
		>
			<option value="" disabled>
				{#if $project.isLoading}Loading...{:else if mediaFilesForDropdown.length === 0}No Media{:else}Select Media{/if}
			</option>
			{#each mediaFilesForDropdown as mediaFile (mediaFile.path)}
				<option value="{mediaFile.path}">{mediaFile.name}</option>
			{/each}
		</select>

		<!-- Transcription Mode -->
		<select
			class="ui-select text-sm text-gray-700 dark:text-gray-200 flex-shrink-0 w-45"
			bind:value="{transcriptionMode}"
			title="Select Transcription Mode"
		>
			<option value="automatic">Automatic Transcription</option>
			<option value="manual">Manual Transcription</option>
		</select>

		<!-- Speakers Button -->
		<div class="relative inline-flex items-center" title="Configure number of speakers and their names">
			<button class="ui-button-icon flex items-center space-x-0.5 hover-scale-effect" on:click="{openSpeakersModal}">
				<svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
					<path stroke-linecap="round" stroke-linejoin="round" d="M18 7.5v3m0 0v3m0-3h3m-3 0h-3m-2.25-4.125a3.375 3.375 0 1 1-6.75 0 3.375 3.375 0 0 1 6.75 0ZM3 19.235v-.11a6.375 6.375 0 0 1 12.75 0v.109A12.318 12.318 0 0 1 9.374 21c-2.331 0-4.512-.645-6.374-1.766Z" />
				  </svg>
				  <span class="text-xs">Speakers</span> <!-- Shorter Text -->
			  {#if $transcriptStore.speakers.count > 0}
				<span class="absolute -top-0.5 -right-0.5 bg-blue-500 text-white rounded-full text-xxs w-3.5 h-3.5 flex items-center justify-center"> <!-- Adjusted badge size/pos -->
					{$transcriptStore.speakers.count}
				</span>
			  {/if}
			</button>
		  </div>

		<!-- Transcribe Button -->
			<button
				class="ui-button-icon flex items-center space-x-0.5 hover-scale-effect"
				on:click="{handleTranscribeClick}"
				title="{isTranscribeDisabled ? 'Select media first' : 'Transcribe Media'}"
				disabled="{isTranscribeDisabled}"
			>
				{#if $transcriptStore.isTranscribing}
				<svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4 animate-spin">
					<path stroke-linecap="round" stroke-linejoin="round" d="M16.023 9.348h4.992v-.001M2.985 19.644v-4.992m0 0h4.992m-4.993 0 3.181 3.183a8.25 8.25 0 0 0 13.803-3.7M4.031 9.865a8.25 8.25 0 0 1 13.803-3.7l3.181 3.182m0-4.991v4.99" />
				</svg>
				<span class="text-xs">Transcribing...</span>
				{:else}
				<svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
					<path stroke-linecap="round" stroke-linejoin="round" d="M7.5 8.25h9m-9 3H12m-9.75 1.51c0 1.6 1.123 2.994 2.707 3.227 1.129.166 2.27.293 3.423.379.35.026.67.21.865.501L12 21l2.755-4.133a1.14 1.14 0 0 1 .865-.501 48.172 48.172 0 0 0 3.423-.379c1.584-.233 2.707-1.626 2.707-3.228V6.741c0-1.602-1.123-2.995-2.707-3.228A48.394 48.394 0 0 0 12 3c-2.392 0-4.744.175-7.043.513C3.373 3.746 2.25 5.14 2.25 6.741v6.018Z" />
				</svg>
				<span class="text-xs">Transcribe</span>
				{/if}
			</button>
	</div>

	<!-- Right Controls: Layout Settings, Theme Toggle -->
	<div class="flex items-center space-x-1.5 flex-shrink-0">
		<!-- Export Button -->
		<button class="ui-button-icon flex items-center space-x-0.5 hover-scale-effect" on:click="{openExportModal}" title="Export Transcript" disabled="{isExportDisabled}">
		   <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4"> <path stroke-linecap="round" stroke-linejoin="round" d="M3 16.5v2.25A2.25 2.25 0 0 0 5.25 21h13.5A2.25 2.25 0 0 0 21 18.75V16.5m-13.5-9L12 3m0 0 4.5 4.5M12 3v13.5" /> </svg>
		   <span class="text-xs">Export</span>
		</button>
		<!-- Layout Settings Button -->
		<button
			on:click="{openLayoutSettingsModal}"
			class="p-1.5 rounded-full border-0 bg-gray-100 text-gray-700 dark:bg-surface-2 dark:text-gray-300 hover:bg-blue-100 hover:text-blue-500 dark:hover:bg-accent-background-hover dark:hover:text-blue-400 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500 transition-colors transition-transform hover:scale-105"
			title="Change Transcript View Layout"
		>
			{@html LAYOUT_ICON_SVG}
		</button>

		<!-- Theme Toggle Button -->
		 <button on:click="{cycleThemePreference}" class="p-1.5 rounded-full border-0 bg-gray-100 text-gray-700 dark:bg-surface-2 dark:text-gray-300 hover:bg-blue-100 hover:text-blue-500 dark:hover:bg-accent-background-hover dark:hover:text-blue-400 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500 transition-colors transition-transform hover:scale-105" title="{themeTitle}">
			{@html themeIconHtml}
		 </button>
	</div>
</div>

<!-- Modals -->

<SpeakersModal bind:showModal="{isSpeakersModalOpen}" currentSpeakers="{$transcriptStore.speakers}" on:confirm="{handleSpeakersConfirm}" />
<ExportModal
	bind:showModal="{isExportModalOpen}" 
	transcriptPath="{transcriptPathForExport}"
	on:confirm="{handleExportConfirm}"
	on:close={() => isExportModalOpen = false}
/>
<LayoutSettingsModal
	bind:showModal="{isLayoutSettingsModalOpen}"
	currentLayoutKey="{$activeLayout}"
	on:selectLayout="{handleLayoutSelected}"
	on:close={() => isLayoutSettingsModalOpen = false}
/>

<style lang="postcss">
	/* Shared button style */
	.ui-button-icon-no-border {
		@apply inline-flex items-center justify-center p-1.5 text-sm font-medium rounded-md text-gray-700 dark:text-white bg-transparent hover:bg-blue-100 dark:hover:bg-blue-700 hover:text-blue-700 hover:border-blue-500 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500 disabled:opacity-50 disabled:cursor-not-allowed transition-colors dark:disabled:hover:!bg-transparent;
	}
	.ui-button-import {
        @apply w-8 h-8 rounded-full flex items-center justify-center transition-colors;
        @apply bg-transparent;
        @apply text-gray-700 dark:text-white;
        @apply border border-gray-300 dark:border-gray-600;
        @apply hover:bg-blue-100 dark:hover:bg-blue-700;
        @apply hover:text-blue-500 dark:hover:text-blue-400;
        @apply hover:border-blue-500 dark:hover:border-blue-500;
        @apply focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500;
        @apply dark:disabled:hover:!bg-transparent;
    }
	.ui-button-icon:disabled {
		@apply opacity-50 cursor-not-allowed;
	}
	.ui-button-icon svg {
		@apply w-4 h-4; /* Adjusted icon size */
	}

	/* Basic style for the new checkbox */
	.ui-checkbox {
		@apply w-3.5 h-3.5 text-blue-600 bg-gray-100 border-gray-300 rounded focus:ring-blue-500 dark:focus:ring-blue-600 dark:ring-offset-gray-800 focus:ring-2 dark:bg-gray-700 dark:border-gray-600; /* Adjusted size */
	}

	/* Ensure spinner icon gets correct color when disabled */
	.ui-button-icon:disabled .animate-spin {
		@apply text-gray-400 dark:text-gray-500; /* Adjusted disabled spinner color */
	}

	/* Override default dark background for TopBar */
	.dark .dark\:bg-gray-800 {
		 background-color: #1f2937 !important; /* Tailwind gray-800 */
	}
	/* Override default dark border for TopBar */
	 .dark .dark\:border-gray-700 {
		 border-color: #374151 !important; /* Tailwind gray-700 */
	 }

	/* Explicit width classes for select elements */
	.w-40 { width: 10rem; } /* 160px */
	.w-36 { width: 9rem;  } /* 144px */
	.w-32 { width: 8rem;  } /* 128px */
	.w-45 { width: 11.25rem; } /* approx */
	.w-34 { width: 8.5rem; } /* approx */
	.w-28 { width: 7rem; } /* approx */

	/* Tailwind class used in SVG - ensure consistency */
	.w-4 { width: 1rem; } /* 16px */
	.h-4 { height: 1rem; } /* 16px */

	/* Extra small text for speaker count badge */
	.text-xxs {
		font-size: 0.65rem; /* ~10.4px */
		line-height: 0.8rem; /* ~12.8px */
	}
    .hover-scale-effect {
        @apply transition-transform hover:scale-105 disabled:hover:scale-100;
        will-change: transform;
        backface-visibility: hidden;
        transform: translateZ(0);
    }
</style>
