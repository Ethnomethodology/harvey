<!-- src/lib/components/projectview/modals/TranslateModal.svelte -->
<script>
	import { createEventDispatcher, onMount, onDestroy } from 'svelte';
	import { get } from 'svelte/store';
	import { 
        CheckCircle, 
        XCircle, 
        Clock, 
        Loader, 
        AlertTriangle, 
        Languages, 
        X 
    } from 'lucide-svelte';
	import { invoke } from '@tauri-apps/api/core';
	import { languageMap } from '$lib/constants/languageMap.js';
	import { transcriptStore, setRanTranslationInBackground, clearTranslationStatus } from '$lib/stores/transcriptStore.js';
	import { configStatus } from '$lib/stores/configStatusStore.js';
	import { getSelectedTranslationEngine } from '$lib/services/configureActions';
    import { 
        Button, 
        Label, 
        Select, 
        Helper 
    } from 'flowbite-svelte';

	export let availableTranscripts = [];
	export let activeTranscriptPath = null;

	const dispatch = createEventDispatcher();

	let selectedTranscript = '';
	let localModels = [];
	let filteredModels = [];
	let modelOptions = [];
	let selectedModel = '';
	let selectedEngine = 'helsinki';
	let selectedSourceLanguage = 'auto';
	let selectedTargetLanguage = 'en';
	
	function formatDuration(seconds) {
		if (!seconds && seconds !== 0) return '0s';
		const m = Math.floor(seconds / 60);
		const s = Math.floor(seconds % 60);
		if (m === 0) return `${s}s`;
		return `${m}m ${s}s`;
	}

	let elapsedText = '';
	let timerInterval;

	function updateElapsed() {
		if ($transcriptStore.isTranslating && $transcriptStore.translationStartTime) {
			const now = Date.now();
			const diff = Math.floor((now - $transcriptStore.translationStartTime) / 1000);
			elapsedText = formatDuration(diff);
		} else {
			elapsedText = '';
		}
	}

	$: if ($transcriptStore.isTranslating && $transcriptStore.translationStartTime) {
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

	// Mapping for NLLB language options (based on supported list)
	const nllbLanguageOptions = [
		{ value: 'auto', name: 'Auto-detect' },
		...Array.from(languageMap.entries())
			.filter(([code, name]) => code.length === 2) // Keep standard 2-letter codes
			.map(([code, name]) => ({ value: code, name: name }))
			.sort((a, b) => a.name.localeCompare(b.name))
	];

	// Filtered list for target (remove auto-detect)
	const nllbTargetLanguageOptions = nllbLanguageOptions.filter(opt => opt.value !== 'auto');

	// Warning icon logic (same as ProjectView)
	$: hasCriticalConfigIssues = !$configStatus.python_libraries_installed;
	$: hasNonCriticalConfigIssues =
		!hasCriticalConfigIssues &&
		(!$configStatus.hf_token_present ||
			!$configStatus.transcription_models_downloaded ||
			!$configStatus.diarization_model_downloaded ||
			!$configStatus.translation_models_downloaded);

	function handleOpenConfig() {
		dispatch('openConfig');
	}

	function formatModelDisplayName(modelName) {
		const parts = modelName.split('/');
		const baseName = parts[parts.length - 1] || modelName;

		if (baseName.toLowerCase().includes('nllb')) {
			if (baseName.includes('600M')) return "NLLB-200 Distilled (Small & Fast)";
			if (baseName.includes('1.3B')) return "NLLB-200 (Medium)";
			if (baseName.includes('3.3B')) return "NLLB-200 (Large)";
			return baseName;
		}

		if (parts.length === 2) {
			const langParts = parts[1].split('-');
			if (langParts.length >= 3 && langParts[0] === 'opus' && langParts[1] === 'mt') {
				const fromCode = langParts[langParts.length - 2];
				const toCode = langParts[langParts.length - 1];
				const fromLang = languageMap.get(fromCode);
				const toLang = languageMap.get(toCode);
				if (fromLang && toLang) {
					return `${fromLang} to ${toLang}`;
				}
			}
		}
		return modelName;
	}

	let transcriptOptions = [];
	$: if (availableTranscripts.length > 0) {
		transcriptOptions = availableTranscripts.map(t => ({
			value: t.relativePath,
			name: t.name || t.relativePath
		}));

		const activeTranscript = availableTranscripts.find(t => t.path === activeTranscriptPath);
		if (activeTranscript) {
			selectedTranscript = activeTranscript.relativePath;
		} else if (!selectedTranscript || !availableTranscripts.some(t => t.relativePath === selectedTranscript)) {
			selectedTranscript = availableTranscripts[0].relativePath;
		}
	} else {
		transcriptOptions = [];
		selectedTranscript = '';
	}

	$: {
		filteredModels = localModels.filter(m => m.family === selectedEngine);
		if (filteredModels.length > 0) {
			modelOptions = filteredModels.map(model => ({
				value: model.name,
				name: formatModelDisplayName(model.name)
			}));
			if (!selectedModel || !filteredModels.some(m => m.name === selectedModel)) {
				selectedModel = filteredModels[0]?.name || '';
			}
		} else {
			modelOptions = [];
			selectedModel = '';
		}
	}

	$: if (showModal) {
		loadData();
	}

	async function loadData() {
		try {
			const [models, engine] = await Promise.all([
				invoke('get_local_translation_models'),
				getSelectedTranslationEngine()
			]);
            localModels = models;
			selectedEngine = engine || 'helsinki';

			// Smarter default for source language
			const activeTranscript = availableTranscripts.find(t => t.path === activeTranscriptPath);
			if (activeTranscript && activeTranscript.language_code) {
				const langCode = activeTranscript.language_code;
				if (langCode === 'original') {
					selectedSourceLanguage = 'auto';
				} else {
					// Check if it's a valid code we have in our list
					const exists = nllbLanguageOptions.some(opt => opt.value === langCode);
					selectedSourceLanguage = exists ? langCode : 'auto';
				}
			} else {
				selectedSourceLanguage = 'auto';
			}
		} catch (e) {
			console.error("Failed to fetch local translation data:", e);
		}
	}

	onMount(async () => {
		await loadData();
	});

	function handleConfirm() {
		const selectedTranscriptObject = availableTranscripts.find(t => t.relativePath === selectedTranscript);
		
		let sourceLang = 'auto';
		let targetLang = 'auto';

		if (selectedModel.toLowerCase().includes('nllb')) {
			sourceLang = selectedSourceLanguage;
			targetLang = selectedTargetLanguage;
		} else if (selectedModel.includes('-')) {
			const parts = selectedModel.split('-');
			// For Helsinki models like "Helsinki-NLP/opus-mt-en-hi", parts are ["Helsinki-NLP/opus", "mt", "en", "hi"]
			targetLang = parts[parts.length - 1];
			// Source is second to last part
			sourceLang = parts[parts.length - 2] || 'auto';
		}

		dispatch('confirm', {
			transcript: selectedTranscriptObject,
			model: selectedModel,
			sourceLanguage: sourceLang,
			targetLanguage: targetLang
		});
	}

	function handleCancelRequest() {
		dispatch('cancelRequest');
	}

	function handleCloseAndReset() {
		clearTranslationStatus(); // Reset translation-related states
		dispatch('closeAndReset');
	}

	function handleRunInBackgroundAndClose() {
        setRanTranslationInBackground(true);
		dispatch('runInBackgroundAndClose');
	}

	$: showModal = $transcriptStore.showTranslateModal;
	$: isTranslating = $transcriptStore.isTranslating;
	$: jobStatus = $transcriptStore.translationJobStatus;
	$: progressMessage = $transcriptStore.translationProgress.message;
	$: currentErrorMessage = $transcriptStore.translationErrorMessage;

	$: modalTitle = (!isTranslating && jobStatus === null) ? 'Translate Transcript' :
					 (isTranslating && jobStatus === 'initiating') ? 'Initiating Translation' :
					 (isTranslating && jobStatus === 'running') ? 'Translation Status' :
					 (jobStatus === 'cancelling') ? 'Cancelling Job' :
					 (!isTranslating && jobStatus === 'done') ? 'Translation Complete' :
					 (!isTranslating && jobStatus === 'error') ? 'Translation Error' :
					 (!isTranslating && jobStatus === 'cancelled') ? 'Translation Cancelled' :
					 'Translation Status';

	let durationText = '';

	// Watch for completion to calculate duration
	$: if (!isTranslating && jobStatus === 'done') {
		const endTime = Date.now();
		const startTime = $transcriptStore.translationStartTime;
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

	function handleKeydown(event) {
		if (showModal && event.key === 'Escape') {
			if (isTranslating && (jobStatus === 'running' || jobStatus === 'initiating')) {
				event.preventDefault();
				return;
			} else {
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
        aria-labelledby="translate-modal-title"
        on:click={() => {
            if (!(isTranslating && (jobStatus === 'running' || jobStatus === 'initiating'))) {
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
                        {#if !isTranslating && jobStatus === null}
                            <Languages size={20} class="text-blue-600 dark:text-blue-400" />
                        {:else if isTranslating}
                            <Loader size={20} class="text-blue-600 dark:text-blue-400 animate-spin" />
                        {:else if jobStatus === 'done'}
                            <CheckCircle size={20} class="text-green-600 dark:text-green-400" />
                        {:else if jobStatus === 'error'}
                            <XCircle size={20} class="text-red-600 dark:text-red-400" />
                        {:else}
                            <Clock size={20} class="text-orange-600 dark:text-orange-400" />
                        {/if}
                    </div>
                    <h3 id="translate-modal-title" class="text-lg font-bold text-gray-900 dark:text-white">
                        {modalTitle}
                    </h3>
                </div>
                {#if !(isTranslating && (jobStatus === 'running' || jobStatus === 'initiating'))}
                    <button on:click={handleCloseAndReset} class="p-2 text-gray-400 hover:text-gray-600 dark:hover:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-800 rounded-full transition-all" title="Close">
                        <X size={20} />
                    </button>
                {/if}
            </div>

            <div class="p-6 overflow-y-auto max-h-[70vh]">
                {#if !isTranslating && jobStatus === null}
                    <!-- CONFIRM VIEW -->
                    <div class="space-y-5">
                        <div class="space-y-2">
                            <Label for="transcriptSelect">Transcript to Translate</Label>
                            <Select
                                id="transcriptSelect"
                                items={transcriptOptions}
                                bind:value={selectedTranscript}
                                placeholder="Select a Transcript"
                                disabled={availableTranscripts.length === 0}
                            />
                        </div>

                        <div class="space-y-2">
                            {#if modelOptions.length === 0}
                                <div class="bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 p-4 rounded-xl text-center space-y-3">
                                    <p class="text-blue-800 dark:text-blue-300 font-medium">
                                        No {selectedEngine === "helsinki" ? "Helsinki-NLP" : "NLLB"} models available
                                    </p>
                                    <Button color="alternative" size="xs" on:click={handleOpenConfig} title="Go to Configuration">
                                        <AlertTriangle size={14} class="mr-2 {hasCriticalConfigIssues ? 'text-red-500' : hasNonCriticalConfigIssues ? 'text-yellow-500' : 'text-gray-500'}" />
                                        Configure
                                    </Button>
                                </div>
                            {:else}
                                <Label for="modelSelect">Translation Model</Label>
                                <Select
                                    id="modelSelect"
                                    items={modelOptions}
                                    bind:value={selectedModel}
                                    placeholder="Select a Model"
                                />
                            {/if}
                        </div>

                        {#if selectedModel.toLowerCase().includes('nllb')}
                            <div class="grid grid-cols-2 gap-4 p-4 bg-gray-50 dark:bg-gray-800/40 rounded-xl border border-gray-100 dark:border-gray-800">
                                <div class="space-y-2">
                                    <Label>Source Language</Label>
                                    <Select
                                        items={nllbLanguageOptions}
                                        bind:value={selectedSourceLanguage}
                                    />
                                </div>
                                <div class="space-y-2">
                                    <Label>Target Language</Label>
                                    <Select
                                        items={nllbTargetLanguageOptions}
                                        bind:value={selectedTargetLanguage}
                                    />
                                </div>
                            </div>
                        {/if}
                    </div>
                {:else if isTranslating && (jobStatus === 'running' || jobStatus === 'initiating')}
                    <!-- RUNNING OR INITIATING VIEW -->
                    <div class="flex flex-col items-center py-8 space-y-6 text-center">
                        <div class="relative">
                            <div class="w-20 h-20 bg-blue-50 dark:bg-blue-900/20 rounded-full flex items-center justify-center">
                                <Loader size={40} class="text-blue-600 dark:text-blue-400 animate-spin" />
                            </div>
                        </div>
                        
                        <div class="space-y-2 w-full px-4">
                            <p class="text-lg font-bold text-gray-900 dark:text-white">
                                {jobStatus === 'initiating' ? 'Preparing Job...' : 'Translating...'}
                            </p>
                            <p class="text-sm text-gray-500 dark:text-gray-400 h-10 flex items-center justify-center">
                                {progressMessage || 'Processing translation segments...'}
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
                            <p class="text-lg font-bold text-gray-900 dark:text-white">Stopping Translation</p>
                            <p class="text-sm text-gray-500 dark:text-gray-400">Attempting to gracefully cancel the process...</p>
                        </div>
                    </div>
                {:else if !isTranslating && jobStatus === 'done'}
                    <div class="flex flex-col items-center py-8 space-y-4 text-center">
                        <div class="w-20 h-20 bg-green-50 dark:bg-green-900/20 rounded-full flex items-center justify-center">
                            <CheckCircle size={40} class="text-green-600 dark:text-green-400" />
                        </div>
                        <div class="space-y-1">
                            <p class="text-lg font-bold text-gray-900 dark:text-white">Translation Complete!</p>
                            {#if durationText}
                                <p class="text-sm text-gray-500 dark:text-gray-400">Total processing time: {durationText}</p>
                            {/if}
                        </div>
                    </div>
                {:else if !isTranslating && jobStatus === 'cancelled'}
                    <div class="flex flex-col items-center py-8 space-y-4 text-center">
                        <div class="w-20 h-20 bg-orange-50 dark:bg-orange-900/20 rounded-full flex items-center justify-center">
                            <XCircle size={40} class="text-orange-600 dark:text-orange-400" />
                        </div>
                        <div class="space-y-1">
                            <p class="text-lg font-bold text-gray-900 dark:text-white">Translation Cancelled</p>
                            <p class="text-sm text-gray-500 dark:text-gray-400">{progressMessage || 'The job was stopped by user.'}</p>
                        </div>
                    </div>
                {:else if !isTranslating && jobStatus === 'error'}
                    <div class="flex flex-col items-center py-6 space-y-4">
                        <div class="w-16 h-16 bg-red-50 dark:bg-red-900/20 rounded-full flex items-center justify-center">
                            <XCircle size={32} class="text-red-600 dark:text-red-400" />
                        </div>
                        <p class="text-lg font-bold text-gray-900 dark:text-white">An Error Occurred</p>
                        <div class="bg-red-50 dark:bg-red-900/20 border border-red-100 dark:border-red-800 p-4 rounded-xl w-full">
                            <p class="text-xs font-mono text-red-700 dark:text-red-300 break-words whitespace-pre-wrap max-h-40 overflow-y-auto">
                                {currentErrorMessage || 'Unknown error during translation.'}
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
            <div class="px-6 py-4 border-t border-gray-200 dark:border-gray-800 flex justify-end gap-3 bg-gray-50/80 dark:bg-gray-800/80 backdrop-blur-md">
                {#if !isTranslating && jobStatus === null}
                    <Button color="alternative" on:click={handleCloseAndReset} title="Cancel">Cancel</Button>
                    <Button
                        color="blue"
                        on:click={handleConfirm}
                        title="Start Translation"
                        disabled={availableTranscripts.length === 0 || !selectedTranscript || !selectedModel}
                    >
                        Start Translation
                    </Button>
                {:else if isTranslating && (jobStatus === 'running' || jobStatus === 'initiating')}
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
                        title="Stop Translation"
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

<style lang="postcss">
</style>