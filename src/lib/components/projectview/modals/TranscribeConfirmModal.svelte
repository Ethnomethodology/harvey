<!-- src/lib/components/projectview/modals/TranscribeConfirmModal.svelte -->
<script>
	import { createEventDispatcher, onDestroy } from 'svelte';
	import { get } from 'svelte/store';
	import { 
        CheckCircle, 
        XCircle, 
        Clock, 
        Loader, 
        AlertTriangle, 
        ExternalLink, 
        Settings2, 
        X, 
        Mic, 
        UserPen, 
        SlidersHorizontal,
        Cpu,
        PencilLine,
        Plus,
        Minus
    } from '@lucide/svelte';
	import { transcriptStore } from '$lib/stores/transcriptStore.js';
	import { configStatus } from '$lib/stores/configStatusStore.js';
	import SpeakersModal from './SpeakersModal.svelte';
	import AdditionalParametersModal from './AdditionalParametersModal.svelte';
    import ManageModelsModal from './ManageModelsModal.svelte';
	import { getDownloadedModels, getSelectedTranscriptionEngine } from '$lib/services/configureActions.js';
	import { invoke } from '@tauri-apps/api/core';
	import { project as projectMainStore } from '$lib/stores/projectStore.js';
	import { open as openExternal } from '@tauri-apps/plugin-shell';
    import { 
        Modal,
        Input, 
        Label, 
        Select, 
        Button, 
        Helper, 
        Checkbox, 
        Tabs, 
        TabItem,
        Radio,
        Progressbar,
        Alert
    } from 'flowbite-svelte';

	// Props
	export let fileName = '';
	export let downloadedModelsList = [];
	export let languageOptions = []; // Expect languageOptions as a prop, default to empty array
	export let speakers = { count: 0, names: [], translatedNames: [] };
	export let mediaDuration = 0; // Added for manual mode validation
	export let lastSegmentEndTime = 0; // Added for manual mode validation

	const dispatch = createEventDispatcher();

	// Local state for editable fields
	let modalSelectedModel = '';
	let modalTab = 'automatic'; // 'automatic' | 'manual'
	let modalSelectedLanguage = 'auto';
    let modalInitialPrompt = '';
    let modalHotwords = '';
    let showAdditionalParamsModal = false;

	let modalEnableDiarization = false;
	let modalSpeakersConfig = { count: 0, names: [], translatedNames: [] };
	let showNestedSpeakersModal = false;
    let showManageModelsModal = false;

	// Manual Mode State
	let manualSegmentCount = 1;
	let manualSegmentDuration = 60;
	let manualSpeakerMode = 'unassigned';

    function handleManualSegCountInput(e) {
        const val = parseInt(e.target.value);
        if (!isNaN(val) && val > 0 && mediaDuration > 0) {
            manualSegmentDuration = Math.max(1, Math.round(mediaDuration / val));
        }
    }

    function handleManualSegDurationInput(e) {
        const val = parseInt(e.target.value);
        if (!isNaN(val) && val > 0 && mediaDuration > 0) {
            manualSegmentCount = Math.min(100, Math.max(1, Math.round(mediaDuration / val)));
        }
    }

    // Helper functions for custom buttons
    function incrementCount() {
        manualSegmentCount = Math.min(100, manualSegmentCount + 1);
        if (mediaDuration > 0) {
            manualSegmentDuration = Math.max(1, Math.round(mediaDuration / manualSegmentCount));
        }
    }
    function decrementCount() {
        manualSegmentCount = Math.max(1, manualSegmentCount - 1);
        if (mediaDuration > 0) {
            manualSegmentDuration = Math.max(1, Math.round(mediaDuration / manualSegmentCount));
        }
    }
    function incrementDuration() {
        manualSegmentDuration = manualSegmentDuration + 1;
        if (mediaDuration > 0) {
            manualSegmentCount = Math.min(100, Math.max(1, Math.round(mediaDuration / manualSegmentDuration)));
        }
    }
    function decrementDuration() {
        manualSegmentDuration = Math.max(1, manualSegmentDuration - 1);
        if (mediaDuration > 0) {
            manualSegmentCount = Math.min(100, Math.max(1, Math.round(mediaDuration / manualSegmentDuration)));
        }
    }

	// Derived state for manual validation
	$: totalDurationNeeded = manualSegmentCount * manualSegmentDuration;
	// For manual transcription initialization (from this modal), we treat it as creating a new transcript/overwriting.
	// So we validate against total media duration, not remaining space.
	$: isManualDurationValid = totalDurationNeeded <= mediaDuration + 0.001;

	$: manualSpeakerOptions = [
		{ value: 'unassigned', label: 'Unassigned' },
		{ value: 'alternate', label: 'Alternate Speakers', disabled: (modalSpeakersConfig?.names?.length || 0) < 2 },
	];

	function formatDuration(seconds) {
		if (!seconds && seconds !== 0) return '0s';
		const m = Math.floor(seconds / 60);
		const s = Math.floor(seconds % 60);
		if (m === 0) return `${s}s`;
		return `${m}m ${s}s`;
	}

	let durationText = '';
	let elapsedText = '';
	let timerInterval;

	function updateElapsed() {
		if ($transcriptStore.isTranscribing && $transcriptStore.transcriptionStartTime) {
			const now = Date.now();
			const diff = Math.floor((now - $transcriptStore.transcriptionStartTime) / 1000);
			elapsedText = formatDuration(diff);
		} else {
			elapsedText = '';
		}
	}

	$: if ($transcriptStore.isTranscribing && $transcriptStore.transcriptionStartTime) {
		if (!timerInterval) {
			updateElapsed();
			timerInterval = setInterval(updateElapsed, 1000);
		}
	} else {
		if (timerInterval) {
			clearInterval(timerInterval);
			timerInterval = null;
		}
	}

	onDestroy(() => {
		if (timerInterval) clearInterval(timerInterval);
	});

	// Event Handlers
	function handleConfirm() {
		// Start time is now handled by store
		if (modalTab === 'automatic') {
            const selectedModelObj = downloadedModelsList.find(m => m.name === modalSelectedModel);
            const engine = selectedModelObj?.family || 'whisper-cpp';

			dispatch('confirmStart', {
				transcriptionMode: 'automatic',
				selectedModel: modalSelectedModel,
                selectedTranscriptionEngine: engine,
				selectedLanguage: modalSelectedLanguage,
				enableDiarization: modalEnableDiarization,
				speakersConfig: modalSpeakersConfig,
                initialPrompt: modalInitialPrompt,
                hotwords: modalHotwords
			});
		} else {
			dispatch('confirmStart', {
				transcriptionMode: 'manual',
				manualSettings: {
					segmentCount: manualSegmentCount,
					segmentDuration: manualSegmentDuration,
					speakerMode: manualSpeakerMode,
					startTime: 0, // Always start from 0 for new transcript via this modal
				},
				speakersConfig: modalSpeakersConfig, // Pass speakers config even in manual for alternations
			});
		}
	}

	function handleCancelRequest() {
		dispatch('cancelRequest');
	}

	function handleCloseAndReset() {
		dispatch('closeAndReset');
	}

	function handleRunInBackgroundAndClose() {
		dispatch('runInBackgroundAndClose');
	}

	function handleOpenConfig() {
		dispatch('openConfig');
	}

	// --- Reactive Derivations from Store for UI ---
	$: showModal = $transcriptStore.showTranscribeModal;
	$: isTranscribing = $transcriptStore.isTranscribing;
	$: jobStatus = $transcriptStore.transcriptionJobStatus;
	$: progressPercent = $transcriptStore.transcriptionProgress.percent;
	$: progressMessage = $transcriptStore.transcriptionProgress.message;
	$: currentErrorMessage = $transcriptStore.transcriptionErrorMessage;
	$: currentJobId = $transcriptStore.transcriptionJobId;
	$: transcriptionOutputFileName = $transcriptStore.transcriptionOutputFileName;

	// Warning icon logic (same as ProjectView)
	$: hasCriticalConfigIssues = !$configStatus.python_libraries_installed;
	$: hasNonCriticalConfigIssues =
		!hasCriticalConfigIssues &&
		(!$configStatus.hf_token_present ||
			!$configStatus.transcription_models_downloaded ||
			!$configStatus.diarization_model_downloaded ||
			!$configStatus.translation_models_downloaded);

	let isInitialized = false;
	// When the modal is about to show the confirm view, initialize local states from the store
	$: if (showModal && !isTranscribing && jobStatus === null && !isInitialized) {
		modalSelectedModel = $transcriptStore.selectedModelName || '';
        
        // Ensure a valid model is selected if possible
        if (downloadedModelsList.length > 0 && (!modalSelectedModel || !downloadedModelsList.some(m => m.name === modalSelectedModel))) {
            modalSelectedModel = downloadedModelsList[0].name;
        }

		modalSelectedLanguage = $transcriptStore.selectedLanguage || 'auto';
		modalTab = $transcriptStore.transcriptionMode || 'automatic'; // Initialize tab from store

        modalInitialPrompt = $transcriptStore.initialPrompt || '';
        modalHotwords = $transcriptStore.hotwords || '';

		modalEnableDiarization = $transcriptStore.diarizationEnabledForNextJob;
		// Initialize modalSpeakersConfig from the speakers prop (which comes from transcriptStore initially)
		// Use a deep copy to prevent direct mutation of the prop or store value until confirmed.
		modalSpeakersConfig = JSON.parse(JSON.stringify(speakers || { count: 0, names: [], translatedNames: [] }));

		// Initialize manual settings from store if available
		if ($transcriptStore.manualSegmentSettings) {
			manualSegmentDuration = $transcriptStore.manualSegmentSettings.duration || 60;
			manualSpeakerMode = $transcriptStore.manualSegmentSettings.speakerMode || 'unassigned';
			if (manualSpeakerMode === 'unselected') manualSpeakerMode = 'unassigned'; // Migration
		}

		isInitialized = true;
	}

	// Update modalSelectedModel if downloadedModelsList changes and it's empty or no longer valid
	$: if (showModal && downloadedModelsList.length > 0) {
        if (!modalSelectedModel || !downloadedModelsList.some(m => m.name === modalSelectedModel)) {
            modalSelectedModel = downloadedModelsList[0].name;
        }
	}

	// Reset the initialization flag when the modal is closed
	$: if (!showModal) {
		isInitialized = false;
	}

	// --- Title Logic ---
	$: modalTitle =
		!isTranscribing && jobStatus === null
			? 'Transcription Settings'
			: isTranscribing && jobStatus === 'initiating'
				? 'Initiating Transcription'
				: isTranscribing && jobStatus === 'running'
					? 'Transcription Status'
					: jobStatus === 'cancelling'
						? 'Cancelling Job'
						: !isTranscribing && jobStatus === 'done'
							? 'Transcription Complete'
							: !isTranscribing && jobStatus === 'error'
								? 'Transcription Error'
								: !isTranscribing && jobStatus === 'cancelled'
									? 'Transcription Cancelled'
									: 'Transcription Status';

	// Watch for completion to calculate duration
	$: if (!isTranscribing && jobStatus === 'done') {
		const endTime = Date.now();
		const startTime = $transcriptStore.transcriptionStartTime;
		const durationMs = startTime ? endTime - startTime : 0;
		const seconds = Math.floor(durationMs / 1000);
		const minutes = Math.floor(seconds / 60);
		const remainingSeconds = seconds % 60;
		
		if (minutes > 0) {
			durationText = `${minutes}m ${remainingSeconds}s`;
		} else {
			durationText = `${seconds}s`;
		}
	}

	// Keyboard handling
	function handleKeydown(event) {
		if (showModal && event.key === 'Escape') {
			if (isTranscribing && (jobStatus === 'running' || jobStatus === 'initiating')) {
				event.preventDefault();
				return;
			} else if (
				!isTranscribing &&
				(jobStatus === 'done' || jobStatus === 'error' || jobStatus === 'cancelled' || jobStatus === null)
			) {
				handleCloseAndReset();
			}
		}
	}
</script>

{#if showModal}
	<div
		class="fixed inset-0 z-[120] flex items-center justify-center bg-black/60 backdrop-blur-sm p-4"
		role="dialog"
		aria-modal="true"
		aria-labelledby="transcribe-modal-title"
		on:click={() => {
			if (!(isTranscribing && (jobStatus === 'running' || jobStatus === 'initiating'))) {
				handleCloseAndReset();
			}
		}}
		tabindex="-1"
		on:keydown={handleKeydown}
	>
		<div
			class="bg-white dark:bg-gray-900 rounded-xl shadow-2xl w-full max-w-md flex flex-col border border-gray-200 dark:border-gray-800 overflow-hidden"
			role="document"
			tabindex="-1"
			on:click|stopPropagation
		>
            <!-- Header -->
            <div class="px-6 py-5 border-b border-gray-200 dark:border-gray-800 flex justify-between items-center bg-gray-50/50 dark:bg-gray-800/50">
                <div class="flex items-center space-x-3">
                    <div class="p-2 bg-blue-100 dark:bg-blue-900/30 rounded-lg">
                        {#if !isTranscribing && jobStatus === null}
                            <Settings2 size={20} class="text-blue-600 dark:text-blue-400" />
                        {:else if isTranscribing}
                            <Settings2 size={20} class="text-blue-600 dark:text-blue-400" />
                        {:else if jobStatus === 'done'}
                            <CheckCircle size={20} class="text-green-600 dark:text-green-400" />
                        {:else if jobStatus === 'error'}
                            <XCircle size={20} class="text-red-600 dark:text-red-400" />
                        {:else}
                            <Clock size={20} class="text-orange-600 dark:text-orange-400" />
                        {/if}
                    </div>
                    <h3 id="transcribe-modal-title" class="text-lg font-bold text-gray-900 dark:text-white">
                        {modalTitle}
                    </h3>
                </div>
                {#if !(isTranscribing && (jobStatus === 'running' || jobStatus === 'initiating'))}
                    <button on:click={handleCloseAndReset} class="p-2 text-gray-400 hover:text-gray-600 dark:hover:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-800 rounded-full transition-all" title="Close">
                        <X size={20} />
                    </button>
                {/if}
            </div>

            <div class="p-6 overflow-y-auto max-h-[70vh]">
                {#if !isTranscribing && jobStatus === null}
                    <!-- CONFIRM VIEW -->
                    <div class="space-y-4">
                        <div class="bg-gray-50 dark:bg-gray-800/40 p-3 rounded-lg border border-gray-100 dark:border-gray-800">
                            <p class="text-xs text-gray-500 dark:text-gray-400 mb-1">Target File</p>
                            <p class="text-sm font-mono text-gray-900 dark:text-gray-200 break-all">{fileName || 'N/A'}</p>
                        </div>

                        <!-- Tabs -->
                        <div class="border-b border-gray-200 dark:border-gray-700 mt-4">
                            <ul class="flex flex-wrap -mb-px text-sm font-medium text-center text-gray-500 dark:text-gray-400">
                                <li class="me-2">
                                    <button 
                                        type="button"
                                        on:click={() => (modalTab = 'automatic')}
                                        class="inline-flex items-center justify-center p-4 border-b-2 rounded-t-lg group transition-all {modalTab === 'automatic' ? 'text-blue-600 border-blue-600 active dark:text-blue-500 dark:border-blue-500' : 'border-transparent hover:text-gray-600 hover:border-gray-300 dark:hover:text-gray-300'}"
                                        title="Automatic Transcription"
                                    >
                                        <Cpu size={18} class="me-2 {modalTab === 'automatic' ? 'text-blue-600 dark:text-blue-500' : 'text-gray-400 group-hover:text-gray-500 dark:text-gray-500 dark:group-hover:text-gray-300'}" />
                                        Automatic
                                    </button>
                                </li>
                                <li class="me-2">
                                    <button 
                                        type="button"
                                        on:click={() => (modalTab = 'manual')}
                                        class="inline-flex items-center justify-center p-4 border-b-2 rounded-t-lg group transition-all {modalTab === 'manual' ? 'text-blue-600 border-blue-600 active dark:text-blue-500 dark:border-blue-500' : 'border-transparent hover:text-gray-600 hover:border-gray-300 dark:hover:text-gray-300'}"
                                        title="Manual Transcription"
                                    >
                                        <PencilLine size={18} class="me-2 {modalTab === 'manual' ? 'text-blue-600 dark:text-blue-500' : 'text-gray-400 group-hover:text-gray-500 dark:text-gray-500 dark:group-hover:text-gray-300'}" />
                                        Manual
                                    </button>
                                </li>
                            </ul>
                        </div>

                        <div class="mt-6">
                            {#if modalTab === 'automatic'}
                                <div class="space-y-4">
                                    {#if hasCriticalConfigIssues}
                                        <div class="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 p-4 rounded-xl text-center space-y-3">
                                            <p class="text-red-800 dark:text-red-300 font-medium">Required libraries missing</p>
                                            <Button color="red" size="xs" on:click={handleOpenConfig} title="Go to Configuration">
                                                Go to Configuration
                                            </Button>
                                        </div>
                                    {:else if downloadedModelsList.length === 0}
                                        <div class="bg-yellow-50 dark:bg-yellow-900/20 border border-yellow-200 dark:border-yellow-800 p-4 rounded-xl text-center space-y-3">
                                            <p class="text-yellow-800 dark:text-yellow-300 font-medium">No transcription models available</p>
                                            <Button color="yellow" size="xs" on:click={() => showManageModelsModal = true} title="Manage Models">
                                                <Settings2 size={14} class="mr-2" />
                                                Manage Models
                                            </Button>
                                        </div>
                                    {:else}
                                        <div class="space-y-2">
                                            <div class="flex items-center justify-between">
                                                <Label for="modalModelSelect" color={!modalSelectedModel ? 'red' : 'gray'}>Transcription Model</Label>
                                                {#if modalSelectedModel}
                                                    {@const selectedModelObj = downloadedModelsList.find(m => m.name === modalSelectedModel)}
                                                    {#if selectedModelObj?.info_url}
                                                        <button 
                                                            class="text-[10px] text-blue-600 dark:text-blue-400 hover:underline flex items-center space-x-1"
                                                            on:click|stopPropagation={() => openExternal(selectedModelObj.info_url)}
                                                            title="View on Hugging Face"
                                                        >
                                                            <span>Hugging Face</span>
                                                            <ExternalLink size={10} />
                                                        </button>
                                                    {/if}
                                                {/if}
                                            </div>
                                            <Select
                                                id="modalModelSelect"
                                                items={downloadedModelsList.map((m) => ({ value: m.name, name: `${m.name} (${m.family || 'whisper.cpp'})` }))}
                                                bind:value={modalSelectedModel}
                                                placeholder="Select a model"
                                            />
                                            {#if !modalSelectedModel}
                                                <div class="flex items-center space-x-2 text-red-600 dark:text-red-400 mt-1 animate-pulse">
                                                    <AlertTriangle size={14} />
                                                    <Helper color="red" class="italic text-[11px] font-medium">Please select a model from the list above to proceed.</Helper>
                                                </div>
                                            {/if}
                                            {#if modalSelectedModel}
                                                {@const selectedModelObj = downloadedModelsList.find(m => m.name === modalSelectedModel)}
                                                {#if selectedModelObj?.description}
                                                    <Helper class="italic text-[11px]">{selectedModelObj.description}</Helper>
                                                {/if}
                                            {/if}
                                        </div>

                                        <div class="space-y-2">
                                            <Label for="modalLanguageSelect">Language</Label>
                                            <Select
                                                id="modalLanguageSelect"
                                                items={languageOptions.map(opt => ({ value: opt.value, name: opt.label }))}
                                                bind:value={modalSelectedLanguage}
                                            />
                                        </div>

                                        <div class="pt-4 border-t border-gray-100 dark:border-gray-800 space-y-4">
                                            <div class="flex justify-between items-center bg-gray-50 dark:bg-gray-800/40 p-3 rounded-lg border border-gray-100 dark:border-gray-800">
                                                <div>
                                                    <p class="text-xs text-gray-500 dark:text-gray-400">Configured Speakers</p>
                                                    <p class="text-sm font-bold text-gray-900 dark:text-white">{modalSpeakersConfig?.count > 0 ? modalSpeakersConfig.count : '0'}</p>
                                                </div>
                                                <Button color="alternative" size="xs" on:click={() => (showNestedSpeakersModal = true)} title="Edit Speakers">
                                                    <UserPen size={14} class="mr-1.5" />
                                                    Edit
                                                </Button>
                                            </div>

                                            {#if $configStatus.diarization_model_downloaded}
                                                <div class="flex items-start space-x-3 p-1">
                                                    <Checkbox bind:checked={modalEnableDiarization} id="diarize-check" class="mt-0.5" />
                                                    <div class="space-y-1 w-full">
                                                        <Label for="diarize-check" class="cursor-pointer">Identify different speakers (diarize)</Label>
                                                        {#if modalEnableDiarization}
                                                            <Alert color="yellow" class="mt-2 py-2 px-3 text-[11px] border border-yellow-200 dark:border-yellow-900/50 bg-yellow-50/50 dark:bg-yellow-900/20" rounded={false}>
                                                                <div class="flex items-center gap-2">
                                                                    <AlertTriangle size={14} class="flex-shrink-0 text-yellow-600 dark:text-yellow-400" />
                                                                    <span class="text-yellow-800 dark:text-yellow-300">Note: This significantly increases processing time.</span>
                                                                </div>
                                                            </Alert>
                                                        {/if}
                                                    </div>
                                                </div>
                                            {:else}
                                                <div class="bg-gray-50 dark:bg-gray-800/40 p-3 rounded-lg border border-dashed border-gray-200 dark:border-gray-700 flex justify-between items-center">
                                                    <div class="space-y-1">
                                                        <p class="text-xs font-medium text-gray-500 dark:text-gray-400">Speaker identification disabled</p>
                                                        <p class="text-[10px] text-gray-400 italic">Download diarization model to enable.</p>
                                                    </div>
                                                    <Button color="alternative" size="xs" on:click={handleOpenConfig} title="Configure Diarization">
                                                        Configure
                                                    </Button>
                                                </div>
                                            {/if}
                                        </div>

                                        <div class="pt-2">
                                            <Button color="alternative" class="w-full" size="sm" on:click={() => (showAdditionalParamsModal = true)} title="Additional Parameters">
                                                <SlidersHorizontal size={14} class="mr-2" />
                                                Additional Parameters
                                            </Button>
                                        </div>
                                    {/if}
                                </div>
                            {:else}
                                <div class="space-y-4">
                                    <div class="grid grid-cols-2 gap-4">
                                        <!-- Segments Input with custom buttons -->
                                        <div class="space-y-2">
                                            <Label for="manualSegCount">Segments</Label>
                                            <div class="relative flex items-center w-full">
                                                <button 
                                                    type="button" 
                                                    on:click={decrementCount}
                                                    class="flex-shrink-0 bg-gray-100 dark:bg-gray-700 dark:hover:bg-gray-600 dark:border-gray-600 hover:bg-gray-200 border border-gray-300 rounded-s-lg p-2 h-9 focus:ring-gray-100 dark:focus:ring-gray-700 focus:ring-2 focus:outline-none transition-colors"
                                                >
                                                    <Minus size={14} class="text-gray-900 dark:text-white" />
                                                </button>
                                                <input 
                                                    type="text" 
                                                    id="manualSegCount" 
                                                    class="bg-gray-50 border-x-0 border-gray-300 h-9 text-center text-gray-900 text-sm focus:ring-blue-500 focus:border-blue-500 block w-full py-2 dark:bg-gray-700 dark:border-gray-600 dark:placeholder-gray-400 dark:text-white dark:focus:ring-blue-500 dark:focus:border-blue-500" 
                                                    value={manualSegmentCount}
                                                    on:input={handleManualSegCountInput}
                                                    required 
                                                    autocomplete="off"
                                                    autocorrect="off"
                                                />
                                                <button 
                                                    type="button" 
                                                    on:click={incrementCount}
                                                    class="flex-shrink-0 bg-gray-100 dark:bg-gray-700 dark:hover:bg-gray-600 dark:border-gray-600 hover:bg-gray-200 border border-gray-300 rounded-e-lg p-2 h-9 focus:ring-gray-100 dark:focus:ring-gray-700 focus:ring-2 focus:outline-none transition-colors"
                                                >
                                                    <Plus size={14} class="text-gray-900 dark:text-white" />
                                                </button>
                                            </div>
                                        </div>

                                        <!-- Duration Input with custom buttons -->
                                        <div class="space-y-2">
                                            <Label for="manualSegDuration">Duration (sec)</Label>
                                            <div class="relative flex items-center w-full">
                                                <button 
                                                    type="button" 
                                                    on:click={decrementDuration}
                                                    class="flex-shrink-0 bg-gray-100 dark:bg-gray-700 dark:hover:bg-gray-600 dark:border-gray-600 hover:bg-gray-200 border border-gray-300 rounded-s-lg p-2 h-9 focus:ring-gray-100 dark:focus:ring-gray-700 focus:ring-2 focus:outline-none transition-colors"
                                                >
                                                    <Minus size={14} class="text-gray-900 dark:text-white" />
                                                </button>
                                                <input 
                                                    type="text" 
                                                    id="manualSegDuration" 
                                                    class="bg-gray-50 border-x-0 border-gray-300 h-9 text-center text-gray-900 text-sm focus:ring-blue-500 focus:border-blue-500 block w-full py-2 dark:bg-gray-700 dark:border-gray-600 dark:placeholder-gray-400 dark:text-white dark:focus:ring-blue-500 dark:focus:border-blue-500" 
                                                    value={manualSegmentDuration}
                                                    on:input={handleManualSegDurationInput}
                                                    required 
                                                    autocomplete="off"
                                                    autocorrect="off"
                                                />
                                                <button 
                                                    type="button" 
                                                    on:click={incrementDuration}
                                                    class="flex-shrink-0 bg-gray-100 dark:bg-gray-700 dark:hover:bg-gray-600 dark:border-gray-600 hover:bg-gray-200 border border-gray-300 rounded-e-lg p-2 h-9 focus:ring-gray-100 dark:focus:ring-gray-700 focus:ring-2 focus:outline-none transition-colors"
                                                >
                                                    <Plus size={14} class="text-gray-900 dark:text-white" />
                                                </button>
                                            </div>
                                        </div>
                                    </div>

                                    <div class="bg-gray-50 dark:bg-gray-800/40 p-3 rounded-lg border border-gray-100 dark:border-gray-800 text-xs space-y-2">
                                        <div class="flex justify-between">
                                            <span class="text-gray-500">Media Duration:</span>
                                            <span class="font-medium text-gray-900 dark:text-white">{formatDuration(mediaDuration)}</span>
                                        </div>
                                        <div class="flex justify-between items-center">
                                            <span class="text-gray-500">Total Segmented:</span>
                                            <span class="text-sm font-bold {isManualDurationValid ? 'text-blue-600 dark:text-blue-400' : 'text-red-600 dark:text-red-400'}">
                                                {formatDuration(totalDurationNeeded)}
                                            </span>
                                        </div>
                                        {#if !isManualDurationValid}
                                            <div class="pt-2 border-t border-red-100 dark:border-red-900/30 text-center">
                                                <p class="font-bold text-red-600 dark:text-red-400">Exceeds media duration!</p>
                                            </div>
                                        {/if}
                                    </div>

                                    <div class="space-y-3 pt-2">
                                        <Label>Speaker Assignment</Label>
                                        <div class="grid grid-cols-2 gap-2">
                                            {#each manualSpeakerOptions as option}
                                                <Radio
                                                    name="manualSpeakerMode"
                                                    value={option.value}
                                                    bind:group={manualSpeakerMode}
                                                    disabled={option.disabled}
                                                    class="p-2 rounded-lg border border-gray-100 dark:border-gray-800 bg-gray-50/50 dark:bg-gray-800/30"
                                                >
                                                    {option.label}
                                                </Radio>
                                            {/each}
                                        </div>
                                        {#if modalSpeakersConfig.names.length < 2}
                                            <Helper class="italic text-[11px]">Note: 'Alternate Speakers' requires at least 2 speakers.</Helper>
                                        {/if}
                                    </div>

                                    <div class="flex justify-between items-center bg-gray-50 dark:bg-gray-800/40 p-3 rounded-lg border border-gray-100 dark:border-gray-800 mt-2">
                                        <div>
                                            <p class="text-xs text-gray-500 dark:text-gray-400">Configured Speakers</p>
                                            <p class="text-sm font-bold text-gray-900 dark:text-white">{modalSpeakersConfig?.count > 0 ? modalSpeakersConfig.count : '0'}</p>
                                        </div>
                                        <button color="alternative" size="xs" on:click={() => (showNestedSpeakersModal = true)} title="Edit Speakers" class="text-xs flex items-center px-2.5 py-1.5 font-medium text-gray-900 focus:outline-none bg-white rounded-lg border border-gray-200 hover:bg-gray-100 hover:text-blue-700 focus:z-10 focus:ring-4 focus:ring-gray-100 dark:focus:ring-gray-700 dark:bg-gray-800 dark:text-gray-400 dark:border-gray-600 dark:hover:text-white dark:hover:bg-gray-700">
                                            <UserPen size={14} class="mr-1.5" />
                                            Edit
                                        </button>
                                    </div>
                                </div>
                            {/if}
                        </div>
                    </div>
                {:else if isTranscribing && (jobStatus === 'running' || jobStatus === 'initiating')}
                    <!-- RUNNING OR INITIATING VIEW -->
                    <div class="flex flex-col items-center py-8 space-y-6 text-center">
                        <div class="relative">
                            <div class="w-20 h-20 bg-blue-50 dark:bg-blue-900/20 rounded-full flex items-center justify-center">
                                <Loader size={40} class="text-blue-600 dark:text-blue-400 animate-spin" />
                            </div>
                        </div>
                        
                        <div class="space-y-2 w-full px-4">
                            <p class="text-lg font-bold text-gray-900 dark:text-white">
                                {jobStatus === 'initiating' ? 'Preparing Job...' : 'Transcribing...'}
                            </p>
                            <p class="text-sm text-gray-500 dark:text-gray-400 h-10 flex items-center justify-center">
                                {progressMessage || 'Processing audio segments...'}
                            </p>
                        </div>

                        {#if elapsedText}
                            <div class="bg-gray-100 dark:bg-gray-800 px-4 py-2 rounded-full font-mono text-xs text-gray-600 dark:text-gray-400">
                                Elapsed: {elapsedText}
                            </div>
                        {/if}
                    </div>
                {:else if jobStatus === 'cancelling'}
                    <div class="flex flex-col items-center py-12 space-y-4 text-center">
                        <Clock size={48} class="text-orange-500 animate-pulse" />
                        <div class="space-y-1">
                            <p class="text-lg font-bold text-gray-900 dark:text-white">Stopping Transcription</p>
                            <p class="text-sm text-gray-500 dark:text-gray-400">Attempting to gracefully cancel the process...</p>
                        </div>
                    </div>
                {:else if !isTranscribing && jobStatus === 'done'}
                    <div class="flex flex-col items-center py-8 space-y-4 text-center">
                        <div class="w-20 h-20 bg-green-50 dark:bg-green-900/20 rounded-full flex items-center justify-center">
                            <CheckCircle size={40} class="text-green-600 dark:text-green-400" />
                        </div>
                        <div class="space-y-1">
                            <p class="text-lg font-bold text-gray-900 dark:text-white">Job Completed!</p>
                            {#if transcriptionOutputFileName}
                                <div class="flex flex-col items-center">
                                    <p class="text-xs font-medium text-green-600 dark:text-green-400 mt-2 bg-green-50/50 dark:bg-green-900/10 px-3 py-1 rounded-full border border-green-100/50 dark:border-green-800/20 max-w-[280px] truncate" title={transcriptionOutputFileName}>
                                        Output: {transcriptionOutputFileName}
                                    </p>
                                </div>
                            {/if}
                            {#if durationText}
                                <p class="text-sm text-gray-500 dark:text-gray-400">Total processing time: {durationText}</p>
                            {/if}
                        </div>
                    </div>
                {:else if !isTranscribing && jobStatus === 'cancelled'}
                    <div class="flex flex-col items-center py-8 space-y-4 text-center">
                        <div class="w-20 h-20 bg-orange-50 dark:bg-orange-900/20 rounded-full flex items-center justify-center">
                            <XCircle size={40} class="text-orange-600 dark:text-orange-400" />
                        </div>
                        <div class="space-y-1">
                            <p class="text-lg font-bold text-gray-900 dark:text-white">Transcription Cancelled</p>
                            <p class="text-sm text-gray-500 dark:text-gray-400">{progressMessage || 'The job was stopped by user.'}</p>
                        </div>
                    </div>
                {:else if !isTranscribing && jobStatus === 'error'}
                    <div class="flex flex-col items-center py-6 space-y-4">
                        <div class="w-16 h-16 bg-red-50 dark:bg-red-900/20 rounded-full flex items-center justify-center">
                            <XCircle size={32} class="text-red-600 dark:text-red-400" />
                        </div>
                        <p class="text-lg font-bold text-gray-900 dark:text-white">An Error Occurred</p>
                        <div class="bg-red-50 dark:bg-red-900/20 border border-red-100 dark:border-red-800 p-4 rounded-xl w-full">
                            <p class="text-xs font-mono text-red-700 dark:text-red-300 break-words whitespace-pre-wrap max-h-40 overflow-y-auto">
                                {currentErrorMessage || 'Unknown error during transcription.'}
                            </p>
                        </div>
                    </div>
                {:else}
                    <div class="flex flex-col items-center py-12 space-y-4">
                        <Loader size={32} class="text-gray-400 animate-spin" />
                        <p class="text-sm text-gray-500">Checking status...</p>
                    </div>
                {/if}
            </div>

            <!-- Footer -->
            <div class="px-6 py-4 border-t border-gray-200 dark:border-gray-800 flex justify-between gap-3 bg-gray-50/80 dark:bg-gray-800/80 backdrop-blur-md">
                {#if !isTranscribing && jobStatus === null}
                    <button on:click={() => showManageModelsModal = true} class="text-sm text-blue-600 dark:text-blue-400 hover:underline flex items-center gap-1.5" title="Manage Transcription Models">
                        <Settings2 size={14} />
                        Manage Models
                    </button>
                    <div class="flex gap-3">
                        <Button color="alternative" on:click={handleCloseAndReset} title="Cancel">Cancel</Button>
                        <Button
                            color="blue"
                            on:click={handleConfirm}
                            title={!modalSelectedModel ? 'Please select a model' : 'Start Transcription'}
                            disabled={(modalTab === 'automatic' && (!modalSelectedModel || !modalSelectedLanguage)) ||
                                (modalTab === 'manual' && !isManualDurationValid)}
                        >
                            {modalTab === 'automatic' ? 'Start' : 'Add Segments'}
                        </Button>
                    </div>
                {:else if isTranscribing && (jobStatus === 'running' || jobStatus === 'initiating')}
                    <Button
                        color="alternative"
                        on:click={handleRunInBackgroundAndClose}
                        disabled={jobStatus === 'initiating'}
                        title="Run in Background"
                    >
                        Run in Background
                    </Button>
                    <Button 
                        color="red" 
                        on:click={handleCancelRequest} 
                        disabled={jobStatus === 'initiating'}
                        title="Stop Transcription"
                    >
                        Stop
                    </Button>
                {:else if jobStatus === 'cancelling'}
                    <Button color="alternative" disabled title="Stopping...">Stopping...</Button>
                {:else}
                    <Button color="blue" on:click={handleCloseAndReset} title="Close">Close</Button>
                {/if}
            </div>
		</div>
	</div>
{/if}

{#if showNestedSpeakersModal}
	<SpeakersModal
		bind:showModal={showNestedSpeakersModal}
		currentSpeakers={modalSpeakersConfig}
		on:confirm={(e) => {
			modalSpeakersConfig = e.detail; // Update local config
			showNestedSpeakersModal = false;
		}}
		on:close={() => (showNestedSpeakersModal = false)}
	/>
{/if}

{#if showAdditionalParamsModal}
    <AdditionalParametersModal
        bind:showModal={showAdditionalParamsModal}
        currentEngine={downloadedModelsList.find(m => m.name === modalSelectedModel)?.family || 'whisper-cpp'}
        initialPrompt={modalInitialPrompt}
        hotwords={modalHotwords}
        on:confirm={async (e) => {
            modalInitialPrompt = e.detail.initialPrompt;
            modalHotwords = e.detail.hotwords;

            // 1. Update store
            transcriptStore.update(ts => ({ ...ts, initialPrompt: modalInitialPrompt, hotwords: modalHotwords }));

            // 2. Update DB
            try {
                const projectData = get(projectMainStore);
                const relativePath = $transcriptStore.selectedMediaFile?.relative_path;

                if (projectData && projectData.id && relativePath) {
                    await invoke('save_media_additional_parameters', {
                        projectId: projectData.id,
                        assetRelativePath: relativePath,
                        initialPrompt: modalInitialPrompt,
                        hotwords: modalHotwords
                    });
                }
            } catch (err) {
                console.error("Failed to save additional parameters immediately:", err);
            }

            showAdditionalParamsModal = false;
        }}
        on:close={() => (showAdditionalParamsModal = false)}
    />
{/if}

<ManageModelsModal
	bind:showModal={showManageModelsModal}
	on:modelsChanged={async () => {
		downloadedModelsList = await getDownloadedModels();
        // Force re-select the first model if the previously selected one was deleted or hidden
        if (downloadedModelsList.length > 0) {
            if (!downloadedModelsList.some(m => m.name === modalSelectedModel)) {
                modalSelectedModel = downloadedModelsList[0].name;
            }
        } else {
            modalSelectedModel = '';
        }
	}}
/>

<style lang="postcss">
    /* Re-enable spin buttons for specific inputs */
    :global(input.show-spinners::-webkit-outer-spin-button),
    :global(input.show-spinners::-webkit-inner-spin-button) {
        -webkit-appearance: inner-spin-button !important;
        opacity: 1 !important;
    }

    :global(input.show-spinners[type=number]) {
        -moz-appearance: number-input !important;
    }
</style>