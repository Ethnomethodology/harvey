<script>
	import { createEventDispatcher } from 'svelte';
	import { X } from 'lucide-svelte';

	export let showModal = false;
	export let currentEngine = 'whisper-cpp';
	export let initialPrompt = '';
	export let hotwords = '';

	const dispatch = createEventDispatcher();

	let localInitialPrompt = '';
	let localHotwords = '';

	$: if (showModal) {
		localInitialPrompt = initialPrompt;
		localHotwords = hotwords;
	}

	function handleConfirm() {
		dispatch('confirm', {
			initialPrompt: localInitialPrompt,
			hotwords: localHotwords
		});
		showModal = false;
	}

	function handleClose() {
		showModal = false;
		dispatch('close');
	}
</script>

{#if showModal}
	<div
		class="fixed inset-0 z-[130] flex items-center justify-center bg-black/50 backdrop-blur-sm p-4"
		role="dialog"
		aria-modal="true"
		on:click={handleClose}
	>
		<div
			class="bg-white dark:bg-gray-900 rounded-lg shadow-xl w-full max-w-md flex flex-col max-h-[90vh]"
			on:click|stopPropagation
		>
			<div class="flex items-center justify-between px-4 py-3 border-b border-gray-200 dark:border-gray-700">
				<h3 class="text-lg font-semibold text-gray-800 dark:text-gray-200">Additional Parameters</h3>
				<button class="text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200" on:click={handleClose}>
					<X class="w-5 h-5" />
				</button>
			</div>

			<div class="p-4 space-y-4 overflow-y-auto">
				{#if currentEngine === 'whisper-cpp'}
					<div class="space-y-2">
						<label for="initialPromptInput" class="block text-sm font-medium text-gray-700 dark:text-gray-300">
							Prompt
						</label>
						<textarea
							id="initialPromptInput"
							bind:value={localInitialPrompt}
							rows="3"
							class="ui-input w-full resize-y text-sm p-2"
							placeholder="e.g. Welcome to the AI podcast."
						></textarea>
						<p class="text-xs text-gray-500">
							Guides the model's style and vocabulary. Provide a sample sentence demonstrating desired formatting or technical terms.
						</p>
					</div>
				{:else if currentEngine === 'faster-whisper'}
					<div class="space-y-2">
						<label for="initialPromptInput" class="block text-sm font-medium text-gray-700 dark:text-gray-300">
							Initial Prompt (Context & Formatting)
						</label>
						<textarea
							id="initialPromptInput"
							bind:value={localInitialPrompt}
							rows="2"
							class="ui-input w-full resize-y text-sm p-2"
							placeholder="e.g. Hello. This is Dr. Smith from Acme Corp."
						></textarea>
						<p class="text-xs text-gray-500">
							Injects text into the model's context window. Best for instructing capitalization, punctuation, and general style.
						</p>
					</div>

					<div class="space-y-2 pt-2 border-t border-gray-100 dark:border-gray-800">
						<label for="hotwordsInput" class="block text-sm font-medium text-gray-700 dark:text-gray-300">
							Hotwords (Jargon & Names)
						</label>
						<textarea
							id="hotwordsInput"
							bind:value={localHotwords}
							rows="2"
							class="ui-input w-full resize-y text-sm p-2"
							placeholder="e.g. dysdiadochokinesia, Smith, Acme Corp"
						></textarea>
						<p class="text-xs text-gray-500">
							Comma-separated list. Explicitly boosts the probability of specific words during decoding to prevent misspellings of hard-to-catch terms.
						</p>
					</div>
				{/if}
			</div>

			<div class="flex justify-end gap-2 px-4 py-3 border-t border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-800 rounded-b-lg">
				<button class="btn-secondary" on:click={handleClose}>Cancel</button>
				<button class="btn-primary" on:click={handleConfirm}>Save</button>
			</div>
		</div>
	</div>
{/if}

<style lang="postcss">
	.ui-input {
		@apply block w-full border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500 sm:text-sm;
		background-color: white;
	}
	:global(.dark) .ui-input {
		background-color: #0d0d0d;
		border-color: #333333;
		color: white;
		color-scheme: dark;
	}
	.btn-primary {
		@apply px-4 py-2 text-sm font-medium text-white bg-blue-600 rounded-md hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 disabled:opacity-50 disabled:cursor-not-allowed;
	}
	.btn-secondary {
		@apply px-4 py-2 text-sm font-medium text-gray-700 bg-white border border-gray-300 rounded-md hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 dark:bg-gray-700 dark:text-gray-200 dark:border-gray-600 dark:hover:bg-gray-600;
	}
</style>