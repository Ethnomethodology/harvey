<!-- src/lib/components/projectview/modals/AdditionalParametersModal.svelte -->
<script>
	import { createEventDispatcher } from 'svelte';
	import { X, SlidersHorizontal } from 'lucide-svelte';
    import { 
        Button, 
        Label, 
        Textarea, 
        Helper 
    } from 'flowbite-svelte';

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
		class="fixed inset-0 z-[130] flex items-center justify-center bg-black/60 backdrop-blur-sm p-4"
		role="dialog"
		aria-modal="true"
		on:click={handleClose}
	>
		<div
			class="bg-white dark:bg-gray-900 rounded-xl shadow-2xl w-full max-w-md flex flex-col border border-gray-200 dark:border-gray-800 overflow-hidden"
			on:click|stopPropagation
		>
            <!-- Header -->
            <div class="px-6 py-5 border-b border-gray-200 dark:border-gray-800 flex justify-between items-center bg-gray-50/50 dark:bg-gray-800/50">
                <div class="flex items-center space-x-3">
                    <div class="p-2 bg-blue-100 dark:bg-blue-900/30 rounded-lg">
                        <SlidersHorizontal size={20} class="text-blue-600 dark:text-blue-400" />
                    </div>
                    <h3 id="additional-params-title" class="text-lg font-bold text-gray-900 dark:text-white">
                        Additional Parameters
                    </h3>
                </div>
                <button on:click={handleClose} class="p-2 text-gray-400 hover:text-gray-600 dark:hover:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-800 rounded-full transition-all" title="Close">
                    <X size={20} />
                </button>
            </div>

			<div class="p-6 space-y-5 overflow-y-auto max-h-[60vh]">
				{#if currentEngine === 'whisper-cpp'}
					<div class="space-y-2">
						<Label for="initialPromptInput">Prompt</Label>
						<Textarea
							id="initialPromptInput"
							bind:value={localInitialPrompt}
							rows="3"
							placeholder="e.g. Welcome to the AI podcast."
						/>
						<Helper class="italic">
							Guides the model's style and vocabulary. Provide a sample sentence demonstrating desired formatting or technical terms.
						</Helper>
					</div>
				{:else if currentEngine === 'faster-whisper'}
					<div class="space-y-2">
						<Label for="initialPromptInput">Initial Prompt (Context & Formatting)</Label>
						<Textarea
							id="initialPromptInput"
							bind:value={localInitialPrompt}
							rows="2"
							placeholder="e.g. Hello. This is Dr. Smith from Acme Corp."
						/>
						<Helper class="italic">
							Injects text into the model's context window. Best for instructing capitalization, punctuation, and general style.
						</Helper>
					</div>

					<div class="space-y-2 pt-4 border-t border-gray-100 dark:border-gray-800">
						<Label for="hotwordsInput">Hotwords (Jargon & Names)</Label>
						<Textarea
							id="hotwordsInput"
							bind:value={localHotwords}
							rows="2"
							placeholder="e.g. dysdiadochokinesia, Smith, Acme Corp"
						/>
						<Helper class="italic">
							Comma-separated list. Explicitly boosts the probability of specific words during decoding to prevent misspellings.
						</Helper>
					</div>
				{/if}
			</div>

            <!-- Footer -->
            <div class="px-6 py-4 border-t border-gray-200 dark:border-gray-800 flex justify-end gap-3 bg-gray-50/80 dark:bg-gray-800/80 backdrop-blur-md">
				<Button color="alternative" on:click={handleClose} title="Cancel">Cancel</Button>
				<Button color="blue" on:click={handleConfirm} title="Save Parameters">Save</Button>
			</div>
		</div>
	</div>
{/if}

<style lang="postcss">
</style>