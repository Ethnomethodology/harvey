<script>
    import { createEventDispatcher, onMount, onDestroy } from 'svelte';
    import { get } from 'svelte/store';
    import { CheckCircle, XCircle, Clock, Loader } from 'lucide-svelte';
    import { invoke } from '@tauri-apps/api/core';
    import Dropdown from '$lib/components/shared/Dropdown.svelte';
    import { languageMap } from '$lib/constants/languageMap.js';
    import { transcriptStore, setRanTranslationInBackground, clearTranslationStatus } from '$lib/stores/transcriptStore.js';
    import { configStatus } from '$lib/stores/configStatusStore.js';
    import { basename } from '@tauri-apps/api/path';
    import { getSelectedTranslationFamily } from '$lib/services/configureActions';
	import { message } from '@tauri-apps/plugin-dialog';

    export let activeDocumentPath = null;
    export let showModal = false;

    const dispatch = createEventDispatcher();

    let localModels = [];
    let filteredModels = [];
    let modelOptions = [];
    let selectedModel = '';
    let selectedFamily = 'helsinki';
    let documentName = '';
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
		{ value: 'auto', label: 'Auto-detect' },
		...Array.from(languageMap.entries())
			.filter(([code, name]) => code.length === 2) // Keep standard 2-letter codes
			.map(([code, name]) => ({ value: code, label: name }))
			.sort((a, b) => a.label.localeCompare(b.label))
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

    $: if (activeDocumentPath) {
        basename(activeDocumentPath).then(name => {
            documentName = name;
        });
    }

    $: {
		filteredModels = localModels.filter(m => m.family === selectedFamily);
        if (filteredModels.length > 0) {
            modelOptions = filteredModels.map(model => ({
                value: model.name,
                label: formatModelDisplayName(model.name)
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
            [localModels, selectedFamily] = await Promise.all([
				invoke('get_local_translation_models'),
				getSelectedTranslationFamily()
			]);
			selectedFamily = selectedFamily || 'helsinki';

			// We don't easily have doc language here yet, so default to auto
			selectedSourceLanguage = 'auto';
        } catch (e) {
            console.error("Failed to fetch local translation data:", e);
        }
    }

    onMount(async () => {
        await loadData();
    });

    function handleConfirm() {
        let sourceLang = 'auto';
		let targetLang = 'auto';

		if (selectedModel.toLowerCase().includes('nllb')) {
			sourceLang = selectedSourceLanguage;
			targetLang = selectedTargetLanguage;
		} else if (selectedModel.includes('-')) {
			const parts = selectedModel.split('-');
			targetLang = parts[parts.length - 1];
			sourceLang = parts[parts.length - 2] || 'auto';
		}

        dispatch('confirm', {
            documentPath: activeDocumentPath,
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

    $: isTranslating = $transcriptStore.isTranslating;
    $: jobStatus = $transcriptStore.translationJobStatus;
    $: progressMessage = $transcriptStore.translationProgress.message;
    $: currentErrorMessage = $transcriptStore.translationErrorMessage;

	let durationText = '';

    $: modalTitle = (!isTranslating && jobStatus === null) ? 'Translate Document' :
                     (isTranslating && jobStatus === 'initiating') ? 'Initiating Translation...' :
                     (isTranslating && jobStatus === 'running') ? 'Translation Status' :
                     (jobStatus === 'cancelling') ? `Cancelling Job` :
                     (!isTranslating && jobStatus === 'done') ? 'Translation Complete' :
                     (!isTranslating && jobStatus === 'error') ? 'Translation Error' :
                     (!isTranslating && jobStatus === 'cancelled') ? 'Translation Cancelled' :
                     'Translation Status';

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
        class="fixed inset-0 z-[120] flex items-center justify-center bg-black/50 backdrop-blur-sm"
        role="dialog"
        aria-modal="true"
        aria-labelledby="translate-doc-modal-title"
        on:click={handleCloseAndReset}
        tabindex="-1"
        on:keydown={handleKeydown}
    >
        <div
            class="bg-white dark:bg-surface-2 rounded-lg shadow-xl p-6 w-full max-w-md text-gray-800 dark:text-gray-200 flex flex-col"
            role="document"
            tabindex="-1"
            on:click|stopPropagation
        >
            <h2 id="translate-doc-modal-title" class="text-lg font-semibold mb-4 text-center">{modalTitle}</h2>

            {#if !isTranslating && jobStatus === null}
                <!-- CONFIRM VIEW -->
                <div class="space-y-4 mb-6">
                    <div class="space-y-1">
                        <label class="block font-medium text-gray-900 dark:text-gray-100">Document:</label>
                        <input 
                            type="text" 
                            disabled 
                            value={documentName}
                            class="w-full px-3 py-2 bg-gray-100 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-md text-gray-500 dark:text-gray-400 cursor-not-allowed"
                        />
                    </div>
                    <div class="space-y-1">
                        {#if localModels.length === 0}
                            <div class="bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 p-4 rounded-md text-center space-y-3 my-4">
                                <p class="text-blue-800 dark:text-blue-300 font-medium">No translation models available.</p>
                                <div class="flex items-center justify-center space-x-2">
                                    <p class="text-xs text-blue-600 dark:text-blue-400">Please download a model in Settings.</p>
                                    <button
                                        type="button"
                                        on:click={handleOpenConfig}
                                        class="flex items-center space-x-1 bg-gray-100 dark:bg-gray-800 px-2 py-1 rounded border border-gray-300 dark:border-gray-600 shadow-sm hover:bg-gray-200 dark:hover:bg-gray-700 transition-colors"
                                    >
                                        <span class="text-xs font-semibold text-gray-700 dark:text-gray-200">Configure</span>
                                    </button>
                                </div>
                            </div>
                        {:else}
                            <label for="modelSelect" class="block font-medium text-gray-900 dark:text-gray-100">Translation Model:</label>
                            <Dropdown
                                containerClasses="w-full"
                                options={modelOptions}
                                bind:value={selectedModel}
                                placeholder="Select a Model"
                            />
                        {/if}
                    </div>

					{#if selectedModel.toLowerCase().includes('nllb')}
						<div class="grid grid-cols-2 gap-4">
							<div class="space-y-1">
								<label class="block font-medium text-gray-900 dark:text-gray-100">From:</label>
								<Dropdown
									containerClasses="w-full"
									options={nllbLanguageOptions}
									bind:value={selectedSourceLanguage}
									placeholder="Source"
								/>
							</div>
							<div class="space-y-1">
								<label class="block font-medium text-gray-900 dark:text-gray-100">To:</label>
								<Dropdown
									containerClasses="w-full"
									options={nllbTargetLanguageOptions}
									bind:value={selectedTargetLanguage}
									placeholder="Target"
								/>
							</div>
						</div>
					{/if}
                </div>
                <div class="flex justify-end space-x-3 mt-auto pt-4 border-t border-gray-200 dark:border-gray-700">
                    <button class="btn-secondary" on:click={handleCloseAndReset}>Cancel</button>
                    <button class="btn-primary" on:click={handleConfirm} disabled={!activeDocumentPath || !selectedModel}>Start Translation</button>
                </div>

            {:else if isTranslating && (jobStatus === 'running' || jobStatus === 'initiating')}
                <!-- RUNNING OR INITIATING VIEW -->
                <div class="flex flex-col items-center space-y-4 mb-6">
                    <div class="w-16 h-16">
                        <Loader class="w-full h-full text-blue-500 animate-spin" />
                    </div>
                    <p class="text-xs text-center text-gray-600 dark:text-gray-400 h-4">
                        {progressMessage || (jobStatus === 'initiating' ? 'Preparing...' : 'Processing...')}
                    </p>
					{#if elapsedText}
						<p class="text-xs text-center text-gray-500 dark:text-gray-500 font-mono mt-1">
							{elapsedText}
						</p>
					{/if}
                </div>
                <div class="flex justify-center space-x-2 mt-auto">
                    <button class="btn-secondary" on:click={handleRunInBackgroundAndClose} disabled={jobStatus === 'initiating'}>
                        Run in background
                    </button>
                    <button class="btn-action-cancel" on:click={handleCancelRequest} disabled={jobStatus === 'initiating'}>
                        Request Cancellation
                    </button>
                </div>

            {:else if jobStatus === 'cancelling'}
                <!-- CANCELLING VIEW -->
                <div class="flex flex-col items-center space-y-4 mb-6">
                    <div class="w-16 h-16">
                        <Clock class="w-full h-full text-orange-500" />
                    </div>
                    <p class="text-xs text-center text-gray-600 dark:text-gray-400 h-4">
                        {progressMessage || 'Attempting to cancel...'}
                    </p>
                </div>
                <div class="flex justify-center space-x-2 mt-auto">
                    <button class="btn-secondary" disabled>Cancelling...</button>
                </div>

            {:else if !isTranslating && jobStatus === 'done'}
                <!-- DONE VIEW -->
                <div class="flex flex-col items-center space-y-3 mb-6 text-center">
                    <CheckCircle class="w-16 h-16 text-green-500" />
                    <p class="text-sm font-medium">Translation Complete!</p>
					{#if durationText}
						<p class="text-xs text-gray-500 dark:text-gray-400">Time taken: {durationText}</p>
					{/if}
                </div>
                <div class="flex justify-center mt-auto">
                    <button class="btn-primary" on:click={handleCloseAndReset}>Close</button>
                </div>

            {:else if !isTranslating && jobStatus === 'cancelled'}
                <!-- CANCELLED VIEW -->
                <div class="flex flex-col items-center space-y-3 mb-6 text-center">
                    <XCircle class="w-16 h-16 text-orange-500" />
                    <p class="text-sm font-medium">{progressMessage || 'Translation Cancelled'}</p>
                </div>
                <div class="flex justify-center mt-auto">
                    <button class="btn-secondary" on:click={handleCloseAndReset}>Close</button>
                </div>

            {:else if !isTranslating && jobStatus === 'error'}
                <!-- ERROR VIEW -->
                <div class="flex flex-col items-center space-y-3 mb-6 text-center">
                    <XCircle class="w-16 h-16 text-red-500" />
                    <p class="text-sm font-medium">An Error Occurred</p>
                    <p class="text-xs bg-red-100 dark:bg-red-900/50 border border-red-300 dark:border-red-700 text-red-700 dark:text-red-300 p-2 rounded w-full text-left overflow-x-auto max-h-32">
                        {currentErrorMessage || 'Unknown error during translation.'}
                    </p>
                </div>
                <div class="flex justify-center mt-auto">
                    <button class="btn-secondary" on:click={handleCloseAndReset}>Close</button>
                </div>
            {:else}
                <!-- Fallback -->
                <div class="flex flex-col items-center space-y-4 py-8">
                    <Loader class="w-12 h-12 text-gray-400 animate-spin" />
                    <p class="text-sm text-gray-500">Loading status...</p>
                </div>
            {/if}
        </div>
    </div>
{/if}

<style>
    .btn-primary,
    .btn-secondary,
    .btn-action-cancel {
        padding: 0.5rem 1rem;
        border: none;
        border-radius: 0.375rem;
        cursor: pointer;
        font-size: 0.875rem;
        font-weight: 500;
        transition: background-color 0.15s ease-in-out, opacity 0.15s ease-in-out;
        white-space: nowrap;
        display: inline-flex;
        align-items: center;
        justify-content: center;
    }
    .btn-primary {
        background-color: #3b82f6;
        color: white;
    }
    .btn-primary:hover:not(:disabled) {
        background-color: #2563eb;
    }
    .btn-primary:disabled {
        opacity: 0.6;
        cursor: not-allowed;
    }
    .btn-secondary {
        background-color: #e5e7eb;
        color: #374151;
        border: 1px solid #d1d5db;
    }
    .dark .btn-secondary {
        background-color: #4b5563;
        color: #e5e7eb;
        border-color: #6b7280;
    }
    .btn-secondary:hover:not(:disabled) {
        background-color: #d1d5db;
    }
    .dark .btn-secondary:hover:not(:disabled) {
        background-color: #6b7280;
    }
    .btn-secondary:disabled {
        opacity: 0.6;
        cursor: not-allowed;
    }
    .btn-action-cancel {
        background-color: #ef4444;
        color: white;
    }
    .btn-action-cancel:hover:not(:disabled) {
        background-color: #dc2626;
    }
    .btn-action-cancel:disabled {
        opacity: 0.6;
        cursor: not-allowed;
    }
</style>
