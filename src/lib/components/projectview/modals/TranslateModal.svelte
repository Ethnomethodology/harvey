<script>
	import { createEventDispatcher, onMount } from 'svelte';

	export let showModal = false;
	export let availableTranscripts = [];

	const dispatch = createEventDispatcher();

	let fromLanguage = 'en';
	let toLanguage = 'ja';
    let selectedTranscript = '';

	const languageOptions = [
		{ value: 'en', label: 'English' },
		{ value: 'ja', label: 'Japanese' },
	];

    onMount(() => {
        console.log("DEBUG: TranslateModal received transcripts:", availableTranscripts);
        if (availableTranscripts.length > 0) {
            selectedTranscript = availableTranscripts[0].relative_path;
        }
    });

	function handleConfirm() {
		// TODO: Implement actual translation logic
		console.log(`Start translation for ${selectedTranscript} from ${fromLanguage} to ${toLanguage}`);
		dispatch('confirm', { transcript: selectedTranscript, from: fromLanguage, to: toLanguage });
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
                {#if availableTranscripts.length === 0}
                    <p class="text-center text-red-500 bg-red-100 dark:bg-red-900/50 p-3 rounded-md">
                        No transcripts found for the selected media file. Please generate a transcript first.
                    </p>
                {:else}
                    <div class="space-y-1">
                        <label for="transcriptSelect" class="block font-medium text-gray-900 dark:text-gray-100">Transcript to Translate:</label>
                        <select id="transcriptSelect" class="ui-select w-full" bind:value={selectedTranscript}>
                            {#each availableTranscripts as transcript, index (transcript.relative_path || `${transcript.name || 'unnamed'}-${index}`)}
                                <option value="{transcript.relative_path}">{transcript.name || transcript.relative_path}</option>
                            {/each}
                        </select>
                    </div>
                {/if}

                <div class="space-y-1">
                    <label for="fromLanguageSelect" class="block font-medium text-gray-900 dark:text-gray-100">Translate From:</label>
                    <select id="fromLanguageSelect" class="ui-select w-full" bind:value={fromLanguage}>
                        {#each languageOptions as lang (lang.value)}
                            <option value="{lang.value}">{lang.label}</option>
                        {/each}
                    </select>
                </div>

                <div class="space-y-1">
                    <label for="toLanguageSelect" class="block font-medium text-gray-900 dark:text-gray-100">Translate To:</label>
                    <select id="toLanguageSelect" class="ui-select w-full" bind:value={toLanguage}>
                        {#each languageOptions as lang (lang.value)}
                            <option value="{lang.value}">{lang.label}</option>
                        {/each}
                    </select>
                </div>
            </div>

            <div class="flex justify-end space-x-3 mt-auto pt-4 border-t border-gray-200 dark:border-gray-700">
                <button class="btn-secondary" on:click={handleClose}>Cancel</button>
                <button class="btn-primary" on:click={handleConfirm} disabled={availableTranscripts.length === 0 || !selectedTranscript}>
                    Start Translation
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