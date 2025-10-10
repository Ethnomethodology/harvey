<script>
	import { createEventDispatcher, onMount } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
	import Dropdown from '$lib/components/shared/Dropdown.svelte';
	import { languageMap } from '$lib/constants/languageMap.js';

	export let showModal = false;
	export let availableTranscripts = [];
	export let isTranslating = false;

	const dispatch = createEventDispatcher();

	let selectedTranscript = '';
	let localModels = [];
	let modelOptions = [];
	let selectedModel = '';
	let translationStatusMessage = '';

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

	$: {
		if (availableTranscripts.length > 0) {
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
	}

	$: {
		if (localModels.length > 0) {
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
	}

	onMount(async () => {
		try {
			localModels = await invoke('get_local_translation_models');
		} catch (e) {
			console.error("Failed to fetch local translation models:", e);
			// Optionally, show an error to the user in the modal
		}
	});

	function handleConfirm() {
		const selectedTranscriptObject = availableTranscripts.find(t => t.relativePath === selectedTranscript);
		dispatch('confirm', {
			transcript: selectedTranscriptObject,
			model: selectedModel,
		});
	}

	function handleClose() {
		dispatch('close');
	}

	function handleKeydown(event) {
		if (event.key === 'Escape') {
			handleClose();
		}
	}
</script>

{#if showModal}
    <div
        class="fixed inset-0 z-[120] flex items-center justify-center bg-black/50 backdrop-blur-sm"
        role="dialog"
        aria-modal="true"
        on:click={handleClose}
        tabindex="-1"
        on:keydown={handleKeydown}
    >
        <div
            class="bg-white dark:bg-gray-800 rounded-lg shadow-xl p-6 w-full max-w-md text-gray-800 dark:text-gray-200 flex flex-col"
            role="document"
            tabindex="-1"
            on:click|stopPropagation
        >
            <h2 class="text-lg font-semibold mb-4 text-center">Translate Transcript</h2>

            <div class="space-y-4 mb-6">
                {#if isTranslating}
                    <div class="flex flex-col items-center justify-center space-y-2">
                        <svg class="animate-spin h-8 w-8 text-blue-500" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                            <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                            <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                        </svg>
                        <p class="text-sm text-gray-500">{translationStatusMessage}</p>
                    </div>
                {:else}
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
                {/if}
            </div>

            <div class="flex justify-end space-x-3 mt-auto pt-4 border-t border-gray-200 dark:border-gray-700">
                <button class="btn-secondary" on:click={handleClose} disabled={isTranslating}>Cancel</button>
                <button class="btn-primary" on:click={handleConfirm} disabled={availableTranscripts.length === 0 || !selectedTranscript || !selectedModel || isTranslating}>
                    {#if isTranslating}Translating...{:else}Start Translation{/if}
                </button>
            </div>
        </div>
    </div>
{/if}

<style>
	.btn-primary, .btn-secondary {
        padding: 0.5rem 1rem;
        border-radius: 0.375rem;
        cursor: pointer;
        font-size: 0.875rem;
        font-weight: 500;
        transition: background-color 0.15s ease-in-out;
        opacity: 1;
    }
    .btn-primary:disabled, .btn-secondary:disabled {
        opacity: 0.5;
        cursor: not-allowed;
    }
    .btn-primary {
        background-color: #3b82f6; color: white;
    }
    .btn-primary:hover:not(:disabled) { background-color: #2563eb; }
    .btn-secondary {
        background-color: #e5e7eb; color: #374151;
        border: 1px solid #d1d5db;
    }
    .dark .btn-secondary { background-color: #4b5563; color: #e5e7eb; border-color: #6b7280; }
    .btn-secondary:hover:not(:disabled) { background-color: #d1d5db; }
    .dark .btn-secondary:hover:not(:disabled) { background-color: #6b7280; }
	.ui-select {
		@apply block w-full pl-3 pr-10 py-2 text-sm border-gray-300 focus:outline-none focus:ring-indigo-500 focus:border-indigo-500 rounded-md dark:bg-gray-700 dark:border-gray-600 dark:placeholder-gray-400 dark:text-white dark:focus:ring-blue-500 dark:focus:border-blue-500;
	}
</style>