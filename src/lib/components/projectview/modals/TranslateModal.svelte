<!-- src/lib/components/projectview/modals/TranslateModal.svelte -->
<script>
	import { createEventDispatcher, onMount, onDestroy } from 'svelte';
	import { get } from 'svelte/store';
	import { CheckCircle, XCircle, Clock, Loader } from 'lucide-svelte';
	import { invoke } from '@tauri-apps/api/core';
	import Dropdown from '$lib/components/shared/Dropdown.svelte';
	import { languageMap } from '$lib/constants/languageMap.js';
	import { transcriptStore, setRanTranslationInBackground, clearTranslationStatus } from '$lib/stores/transcriptStore.js';

	export let availableTranscripts = [];

	const dispatch = createEventDispatcher();

	let selectedTranscript = '';
	let localModels = [];
	let modelOptions = [];
	let selectedModel = '';

	function formatModelDisplayName(modelName) {
		const parts = modelName.split('/');
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
			label: t.name || t.relativePath
		}));
		if (!selectedTranscript || !availableTranscripts.some(t => t.relativePath === selectedTranscript)) {
			selectedTranscript = availableTranscripts[0].relativePath;
		}
	} else {
		transcriptOptions = [];
		selectedTranscript = '';
	}

	$: if (localModels.length > 0) {
		modelOptions = localModels.map(model => ({
			value: model.name,
			label: formatModelDisplayName(model.name)
		}));
		if (!selectedModel || !localModels.some(m => m.name === selectedModel)) {
			selectedModel = localModels[0]?.name || '';
		}
	} else {
		modelOptions = [];
		selectedModel = '';
	}

	onMount(async () => {
		try {
			localModels = await invoke('get_local_translation_models');
		} catch (e) {
			console.error("Failed to fetch local translation models:", e);
		}
	});

	function handleConfirm() {
		const selectedTranscriptObject = availableTranscripts.find(t => t.relativePath === selectedTranscript);
		dispatch('confirm', {
			transcript: selectedTranscriptObject,
			model: selectedModel,
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
	$: progressPercent = $transcriptStore.translationProgress.percent;
	$: progressMessage = $transcriptStore.translationProgress.message;
	$: currentErrorMessage = $transcriptStore.translationErrorMessage;
	$: currentJobId = $transcriptStore.translationJobId;

	$: modalTitle = (!isTranslating && jobStatus === null) ? 'Translate Transcript' :
					 (isTranslating && jobStatus === 'initiating') ? 'Initiating Translation...' :
					 (isTranslating && jobStatus === 'running') ? `Translation Status${currentJobId ? ` (Job: ${currentJobId.substring(0, 8)})` : ''}` :
					 (jobStatus === 'cancelling') ? `Cancelling Job${currentJobId ? ` (${currentJobId.substring(0, 8)})` : ''}` :
					 (!isTranslating && jobStatus === 'done') ? 'Translation Complete' :
					 (!isTranslating && jobStatus === 'error') ? 'Translation Error' :
					 (!isTranslating && jobStatus === 'cancelled') ? 'Translation Cancelled' :
					 'Translation Status';

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
        aria-labelledby="translate-modal-title"
        on:click={handleCloseAndReset}
        tabindex="-1"
        on:keydown={handleKeydown}
    >
        <div
            class="bg-white dark:bg-gray-800 rounded-lg shadow-xl p-6 w-full max-w-md text-gray-800 dark:text-gray-200 flex flex-col"
            role="document"
            tabindex="-1"
            on:click|stopPropagation
        >
            <h2 id="translate-modal-title" class="text-lg font-semibold mb-4 text-center">{modalTitle}</h2>

            {#if !isTranslating && jobStatus === null}
                <!-- CONFIRM VIEW -->
                <div class="space-y-4 mb-6">
                    <div class="space-y-1">
                        <label for="transcriptSelect" class="block font-medium text-gray-900 dark:text-gray-100">Transcript to Translate:</label>
                        <Dropdown
                            containerClasses="w-full"
                            options={transcriptOptions}
                            bind:value={selectedTranscript}
                            placeholder="Select a Transcript"
                            disabled={availableTranscripts.length === 0}
                        />
                    </div>
                    <div class="space-y-1">
                        <label for="modelSelect" class="block font-medium text-gray-900 dark:text-gray-100">Translation Model:</label>
                        <Dropdown
                            containerClasses="w-full"
                            options={modelOptions}
                            bind:value={selectedModel}
                            placeholder={localModels.length === 0 ? "No Models Downloaded" : "Select a Model"}
                            disabled={localModels.length === 0}
                        />
                    </div>
                </div>
                <div class="flex justify-end space-x-3 mt-auto pt-4 border-t border-gray-200 dark:border-gray-700">
                    <button class="btn-secondary" on:click={handleCloseAndReset}>Cancel</button>
                    <button class="btn-primary" on:click={handleConfirm} disabled={availableTranscripts.length === 0 || !selectedTranscript || !selectedModel}>Start Translation</button>
                </div>

            {:else if isTranslating && (jobStatus === 'running' || jobStatus === 'initiating')}
				<!-- RUNNING OR INITIATING VIEW -->
				<div class="flex flex-col items-center space-y-4 mb-6">
                    <div class="w-16 h-16">
                        <Loader class="w-full h-full text-blue-500 animate-spin" />
                    </div>
                    <div class="w-full bg-gray-200 rounded-full h-2.5 dark:bg-gray-700">
                        <div class="bg-blue-600 h-2.5 rounded-full" style="width: {progressPercent}%"></div>
                    </div>
					<p class="text-xs text-center text-gray-600 dark:text-gray-400 h-4">
                        {progressMessage || (jobStatus === 'initiating' ? 'Preparing...' : 'Processing...')}
					</p>
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
					<p class="text-sm font-medium">{progressMessage || 'Translation Complete!'}</p>
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
				<!-- Fallback or initial brief loading state -->
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
		border-radius: 0.375rem; /* 6px */
		cursor: pointer;
		font-size: 0.875rem; /* 14px */
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
    .dark .btn-action-cancel {
         background-color: #dc2626;
    }
    .dark .btn-action-cancel:hover:not(:disabled) {
        background-color: #b91c1c;
    }
</style>
