<!-- src/lib/components/projectview/transcriptions/TopBar.svelte -->
<script>
	import { Button } from 'flowbite-svelte';
	import { MessageSquareText, Share, Languages, Users, LayoutDashboard, SquareSplitHorizontal, SquareSplitVertical, Sun, Moon, Monitor, AudioLines, Rows2 } from 'lucide-svelte';
	// --- Svelte/Store Imports ---
	import { createEventDispatcher, onMount } from 'svelte';
	import { get } from 'svelte/store';
	import { project } from '$lib/stores/projectStore.js'; // For project-level state like isLoading, files, isTranscribing
	import { transcriptStore, setSelectedModel, setSelectedLanguage, updateSpeakerConfig, selectMedia, setTranslateToEnglish, toggleTranslateModal, toggleDualMode, setDualTranscriptModal, deactivateDualMode } from '$lib/stores/transcriptStore.js';
	import { themePreference, cycleThemePreference } from '$lib/stores/themeStore.js';
	import waveformLayoutStore from '$lib/stores/waveformLayoutStore.js';
	import { configStatus, updateConfigStatus } from '$lib/stores/configStatusStore.js';

	// --- Service Imports ---
	import { requestTranscription, requestTranslation } from '$lib/services/projectService.js';
	import { getDownloadedModels, exportTranscript } from '$lib/services/configureActions.js';

	// --- Tauri Imports ---
	import { message } from '@tauri-apps/plugin-dialog';

	// --- Child Component Imports ---
	
	import SpeakersModal from '../modals/SpeakersModal.svelte';
	import ExportModal from '../modals/ExportModal.svelte';
	import LayoutSettingsModal from '../modals/LayoutSettingsModal.svelte';
	import DualTranscriptModal from '../modals/DualTranscriptModal.svelte';
	import { activeLayout } from '$lib/stores/layoutStore.js';
	import { languageOptions } from '$lib/constants/transcriptionOptions.js';
	import Dropdown from '$lib/components/shared/Dropdown.svelte';
    import TranslateModal from '../modals/TranslateModal.svelte';
    import ImportSpeedDial from '$lib/components/projectview/shared/ImportSpeedDial.svelte';

	// --- Local state ---
	const dispatch = createEventDispatcher();
	let downloadedModelsList = [];
	
	let isLoadingModels = true;
	let isManageModalOpen = false;
	let isSpeakersModalOpen = false;
	let isExportModalOpen = false;
	let isLayoutSettingsModalOpen = false; // Added
	let transcriptsForModal = [];

	export function openTranslateModal() {
		const selectedMedia = $transcriptStore.selectedMediaFile;

		if (!selectedMedia?.relative_path) {
			transcriptsForModal = [];
			toggleTranslateModal(true);
			return;
		}

		let foundFile = null;
		function findFileByRelativePath(nodes, relativePath) {
			for (const node of nodes) {
				if (node.relative_path === relativePath) {
					foundFile = node;
					return;
				}
				if (node.children) {
					findFileByRelativePath(node.children, relativePath);
				}
				if (foundFile) return;
			}
		}

		findFileByRelativePath($project.files || [], selectedMedia.relative_path);

		        if (foundFile && foundFile.associated_transcripts) {
		            transcriptsForModal = foundFile.associated_transcripts;
		            console.log("DEBUG: Transcripts for modal:", transcriptsForModal);
		        } else {
		            console.warn("DEBUG: No transcripts found for modal or foundFile missing:", foundFile);
		            transcriptsForModal = [];
		        }
		toggleTranslateModal(true);
	}
	
	// Variable to hold transcript path for export modal
	let transcriptPathForExport = '';

	function handleAddBlankTranscript() {
		console.log('Add Blank Transcript clicked');
		// TODO: implement blank transcript creation
	}

	// --- Load Configuration ---
	async function loadConfiguration() {
        isLoadingModels = true;
        try {
            const localModelsResult = await getDownloadedModels();
            downloadedModelsList = localModelsResult;
            console.log("TopBar: Loaded local models:", downloadedModelsList);
        } catch (e) {
            console.error("TopBar: Error during configuration loading:", e);
            downloadedModelsList = [];
        } finally {
            isLoadingModels = false;
            // validateSelectedModel();

            // --- ADDED: Set Default Model and Language ---
            const currentTranscriptState = get(transcriptStore); // Get current state non-reactively

            // Set default model if none selected
            if (!currentTranscriptState.selectedModelName) {
                let defaultModel = downloadedModelsList[0]?.name; // Try first local model
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
	

	// --- Lifecycle ---
	onMount(async () => { 
		await updateConfigStatus();
		await loadConfiguration(); 
	});

	// --- Event Handlers ---
	async function handleTranscribeClick() {
		console.log('TopBar: Transcribe icon clicked');
		if (!$transcriptStore.selectedMediaFile?.path) {
			message("Please select a media file first.", { title: "No Media Selected", type: "warning" });
			return;
		}
		
		if (!$transcriptStore.selectedLanguage) {
			message("Please select the audio language first.", { title: "No Language Selected", type: "warning" });
			return;
		}
		await requestTranscription(); // This service function will now internally get state from transcriptStore or be passed it
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
		const { filePath, format, layoutChoice, excludeSpeakerNames } = event.detail;
		console.log('TopBar: Export modal confirmed. Exporting:', { filePath, format, layoutChoice, excludeSpeakerNames });
		const segmentsToExport = $transcriptStore.segments;
		if (!segmentsToExport || segmentsToExport.length === 0) {
			console.error("TopBar: Cannot export, no segments available in store.");
			message("No transcript data available to export.", { title: "Export Failed", type: "error" });
			return;
		}
		try {
			await exportTranscript(filePath, format, segmentsToExport, transcriptPathForExport, layoutChoice, excludeSpeakerNames);
			console.log(`TopBar: Export to ${filePath} (${format}, Layout: ${layoutChoice || 'N/A'}, ExcludeSpeakers: ${excludeSpeakerNames}) successful.`);
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
	function handleSpeakersConfirm(event) {
		const { count, names, translatedNames } = event.detail;
		console.log("TopBar: Confirmed speakers:", count, names, translatedNames);
		updateSpeakerConfig(count, names, translatedNames);
	}
	function handleMediaSelectionChange(selectedPath) {
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

	function cycleWaveformLayout() {
		const layouts = ['none', 'horizontal', 'vertical'];
		const currentIndex = layouts.indexOf($waveformLayoutStore);
		const nextIndex = (currentIndex + 1) % layouts.length;
		waveformLayoutStore.setLayout(layouts[nextIndex]);
	}

	async function handleDualModeToggle() {
		if ($transcriptStore.isDualModeActive) {
			// If already active, deactivate
			await deactivateDualMode();
		} else {
			// If not active, check for media first
			if (!$transcriptStore.selectedMediaFile?.path) {
				message("Please select a media file first.", { title: "No Media Selected", type: "warning" });
				return;
			}
			// Open the selection modal
			setDualTranscriptModal(true);
		}
	}

</script>

<!-- Top Bar Structure -->
<div
	class="flex items-center justify-between px-1 h-10 flex-shrink-0 bg-white dark:bg-gray-950 border-b border-gray-200 dark:border-gray-800"
	data-tauri-drag-region
>
	<!-- Left Controls: Toggle Panel, Media Select, Model Select, Language Select, Speakers, Transcribe -->
	<div class="flex items-center space-x-1.5">
		

		<!-- Media Selection Dropdown -->
		<Dropdown
			containerClasses="w-72 ml-2"
			options={mediaFilesForDropdown.map(f => ({ value: f.path, label: f.name }))}
			bind:value={selectedMediaValue}
			on:change={(e) => handleMediaSelectionChange(e.detail)}
			placeholder={$project.isLoading ? 'Loading...' : (mediaFilesForDropdown.length === 0 ? 'No Media' : 'Select Media')}
			disabled={$project.isLoading || mediaFilesForDropdown.length === 0}
		/>

		<!-- Speakers Button -->
		<div class="relative inline-flex items-center ml-2">
			<Button size="xs" color="alternative" class="space-x-0.5 px-2 !py-1 relative" on:click="{openSpeakersModal}" title="Configure number of speakers and their names">
				<Users class="w-3.5 h-3.5" />
				<span>Speakers</span> <!-- Shorter Text -->
			  {#if $transcriptStore.speakers.count > 0}
				<span class="absolute -top-1.5 -right-1.5 bg-blue-500 text-white rounded-full text-xxs w-4 h-4 flex items-center justify-center font-bold"> <!-- Adjusted badge size/pos -->
					{$transcriptStore.speakers.count}
				</span>
			  {/if}
			</Button>
		  </div>

		<!-- Transcribe Button -->
			<Button
				size="xs" color="alternative" class="ml-2 space-x-0.5 px-2 !py-1"
				on:click="{handleTranscribeClick}"
				disabled="{isTranscribeDisabled}"
                title="{isTranscribeDisabled ? 'Select media first' : 'Transcribe Media'}"
			>
				{#if $transcriptStore.isTranscribing}
				<svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-3.5 h-3.5 animate-spin">
					<path stroke-linecap="round" stroke-linejoin="round" d="M16.023 9.348h4.992v-.001M2.985 19.644v-4.992m0 0h4.992m-4.993 0 3.181 3.183a8.25 8.25 0 0 0 13.803-3.7M4.031 9.865a8.25 8.25 0 0 1 13.803-3.7l3.181 3.182m0-4.991v4.99" />
				</svg>
				<span>Transcribing...</span>
				{:else}
				<MessageSquareText class="w-3.5 h-3.5 {$configStatus.transcription_models_downloaded ? '' : 'text-yellow-500'}" />
				<span>Transcribe</span>
				{/if}
			</Button>

			<!-- Translate Button -->
			<Button
				size="xs" color="alternative" class="ml-2 space-x-0.5 px-2 !py-1"
				on:click={openTranslateModal}
                title="Translate Transcript"
			>
				{#if $transcriptStore.isTranslating}
				<svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-3.5 h-3.5 animate-spin">
					<path stroke-linecap="round" stroke-linejoin="round" d="M16.023 9.348h4.992v-.001M2.985 19.644v-4.992m0 0h4.992m-4.993 0 3.181 3.183a8.25 8.25 0 0 0 13.803-3.7M4.031 9.865a8.25 8.25 0 0 1 13.803-3.7l3.181 3.182m0-4.991v4.99" />
				</svg>
				<span>Translating...</span>
				{:else}
				<Languages class="w-3.5 h-3.5" />
				<span>Translate</span>
				{/if}
			</Button>

			
	</div>

	<!-- Right Controls: Layout Settings, Theme Toggle -->
	<div class="flex items-center space-x-1.5 flex-shrink-0">
        <ImportSpeedDial on:requestImportAction />
		<!-- Export Button -->
		<Button size="xs" color="alternative" class="space-x-0.5 px-2 !py-1" on:click="{openExportModal}" disabled="{isExportDisabled}" title="Export Transcript">
		   <Share class="w-3.5 h-3.5" />
		   <span>Export</span>
		</Button>

		<!-- Dual Mode Toggle Button -->
		<button 
			on:click="{handleDualModeToggle}" 
			class="p-1.5 rounded-sm border-0 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500 transition-colors {$transcriptStore.isDualModeActive ? 'bg-blue-100 text-blue-600 dark:bg-blue-500/20 dark:text-blue-400' : 'bg-gray-100 text-gray-700 dark:bg-gray-900 dark:text-gray-300 hover:bg-blue-100 dark:hover:bg-blue-500/10'}"
			title="Dual Transcript Mode"
		>
			<Rows2 size={16} strokeWidth={2} />
		</button>

		<!-- Layout Settings Button -->
		<button
			on:click="{openLayoutSettingsModal}"
			class="p-1.5 rounded-full border-0 bg-gray-100 text-gray-700 dark:bg-gray-900 dark:text-gray-300 hover:bg-blue-100 dark:hover:bg-blue-500/10 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500 transition-colors"
			title="Change Transcript View Layout"
		>
			<LayoutDashboard class="w-4 h-4" />
		</button>

		<!-- Waveform Toggle Button -->
		<button 
			on:click="{cycleWaveformLayout}" 
			class="p-1.5 rounded-full border-0 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500 transition-colors {$waveformLayoutStore === 'none' ? 'bg-gray-100 text-gray-400 dark:bg-gray-900 dark:text-gray-600' : 'bg-blue-100 text-blue-600 dark:bg-blue-500/20 dark:text-blue-400'}"
			title="Toggle Waveform Panel ({$waveformLayoutStore})"
		>
			<div class="transition-transform duration-200" style="transform: rotate({$waveformLayoutStore === 'vertical' ? '90deg' : '0deg'})">
				<AudioLines size={16} strokeWidth={2} />
			</div>
		</button>

		<!-- Theme Toggle Button -->
		 <button on:click="{cycleThemePreference}" class="p-1.5 rounded-full border-0 bg-gray-100 text-gray-700 dark:bg-gray-900 dark:text-gray-300 hover:bg-blue-100 dark:hover:bg-blue-500/10 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500 transition-colors" title="{themeTitle}">
            {#if $themePreference === 'light'}
                <Moon class="w-4 h-4" />
            {:else if $themePreference === 'dark'}
                <Monitor class="w-4 h-4" />
            {:else}
                <Sun class="w-4 h-4" />
            {/if}
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
	hideWaveformOptions={true}
/>

<DualTranscriptModal />

<TranslateModal 
    availableTranscripts={transcriptsForModal}
    activeTranscriptPath={$transcriptStore.currentTranscriptPath}
    on:confirm={async (e) => {
        console.log('Translation confirmed:', e.detail);
        await requestTranslation(e.detail.transcript.path, e.detail.model, e.detail.targetLanguage, e.detail.sourceLanguage);
    }}
    on:cancelRequest={() => dispatch('cancelTranslationRequest')}
    on:openConfig={() => dispatch('openConfig')}
    on:closeAndReset={() => toggleTranslateModal(false)}
    on:runInBackgroundAndClose={() => {
        dispatch('runTranslationInBackground');
        toggleTranslateModal(false);
    }}
/>

<style lang="postcss">
	/* Shared button style */
	.ui-button-icon-no-border {
		@apply inline-flex items-center justify-center p-1.5 text-sm font-medium rounded-md text-gray-700 dark:text-white bg-transparent hover:bg-blue-100 dark:hover:bg-blue-700 hover:border-blue-500 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500 disabled:opacity-50 disabled:cursor-not-allowed transition-colors dark:disabled:hover:!bg-transparent;
	}
	.ui-button-import {
        @apply w-8 h-8 rounded-full flex items-center justify-center transition-colors;
        @apply bg-transparent;
        @apply text-gray-700 dark:text-white;
        @apply border border-gray-300 dark:border-gray-600;
        @apply hover:bg-blue-100 dark:hover:bg-blue-700;
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

	/* Extra small text for speaker count badge */
	.text-xxs {
		font-size: 0.65rem; /* ~10.4px */
		line-height: 0.8rem; /* ~12.8px */
	}
    .hover-scale-effect {
        /* @apply transition-transform hover:scale-105 disabled:hover:scale-100; */
        will-change: transform;
        backface-visibility: hidden;
        transform: translateZ(0);
    }
</style>
