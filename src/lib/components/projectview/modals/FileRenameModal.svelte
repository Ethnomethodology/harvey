<!-- src/lib/components/projectview/modals/FileRenameModal.svelte -->
<script>
	import { createEventDispatcher, onMount, tick } from 'svelte';
	import { fade } from 'svelte/transition';

	export let showModal = false;
	export let currentName = '';
	export let itemType = ''; // 'media', 'transcript', 'note', 'doc', 'other'
    export let isMediaRename = false; // Explicit prop if needed, though itemType usually sufficient

	let newNameBase = '';
	let inputElement;
	let errorMessage = '';

	const dispatch = createEventDispatcher();

	// Determines if the user should only input a base name (stem).
	// 'transcript' here refers to media-associated transcripts which become .json.
	$: isStemInputMode = ['media', 'doc', 'table', 'image', 'imported_transcript', 'note', 'transcript'].includes(itemType);

	let currentBaseName = '';
	let currentExtension = '';
    let currentDisplayName = '';

	function updateNameParts() {
		if (currentName) {
			const lastDotIndex = currentName.lastIndexOf('.');
			if (lastDotIndex > 0 && lastDotIndex < currentName.length - 1) {
				// Found a likely extension
				currentBaseName = currentName.substring(0, lastDotIndex);
				currentExtension = currentName.substring(lastDotIndex);
                // Show only stem for types where user provides stem
                if (isStemInputMode) {
                    currentDisplayName = currentBaseName;
                } else {
                    currentDisplayName = currentName; // Show full name for others by default
                }
			} else {
				// No extension found or dot is at the beginning
				currentBaseName = currentName;
				currentExtension = '';
				currentDisplayName = currentName; // Show the full name if no extension
                // console.warn(`[Rename Modal] Could not extract extension from '${currentName}' for itemType '${itemType}'.`);
			}
		} else {
            // No current name provided
            currentBaseName = '';
            currentExtension = '';
            currentDisplayName = '';
        }
		newNameBase = currentBaseName; // Initialize input with base name
	}

	$: if (currentName || itemType) {
		updateNameParts();
	}

	$: if (showModal) {
		errorMessage = '';
		updateNameParts(); // Ensure parts are recalculated when shown
		tick().then(() => {
			inputElement?.focus();
			inputElement?.select();
		});
	}

	function handleConfirm() {
		const baseNameInput = newNameBase.trim();
		errorMessage = ''; // Clear previous error

		if (!baseNameInput) {
			errorMessage = 'Name cannot be empty.';
		} else if (/[<>:"/\\|?*]/.test(baseNameInput)) {
			errorMessage = 'Name contains invalid characters (< > : " / \\ | ? *).';
		} else if (baseNameInput.startsWith('.')) {
			errorMessage = 'Name cannot start with a dot.';
		}

		if (isStemInputMode) {
			// For stem inputs, disallow dots in the stem itself,
			// except for media-associated 'transcript' type where stem can have dots before .json is added.
			if (baseNameInput.includes('.') && itemType !== 'transcript') {
				errorMessage = `Base name for ${itemType} cannot contain dots. Extension is handled automatically.`;
			}
		} else {
			// User provides full name, must include an extension.
			if (!baseNameInput.includes('.')) {
				errorMessage = 'Filename must include an extension.';
			}
		}

		if (errorMessage) {
			return;
		}

		let nameToSend = '';
		let isSameName = false;

		if (itemType === 'media' || itemType === 'doc' || itemType === 'table' || itemType === 'image' || itemType === 'imported_transcript') {
			nameToSend = baseNameInput; // Send stem
			if (nameToSend === currentBaseName) {
				isSameName = true;
			}
		} else if (itemType === 'note' || itemType === 'transcript') { // 'transcript' here is media-associated
			// These types have a fixed .json extension added to the stem.
			nameToSend = `${baseNameInput}.json`;
			if (nameToSend === currentName) {
				isSameName = true;
			}
		} else { // User provided full name (e.g., a generic file type not explicitly handled as stem input)
			nameToSend = baseNameInput;
			if (nameToSend === currentName) {
				isSameName = true;
			}
		}

		if (isSameName) {
			errorMessage = 'New name is the same as the current name.';
			return;
		}

		console.log(`[Rename Modal] Dispatching confirm. Item Type: '${itemType}', Name to Send: '${nameToSend}' (Original Full: '${currentName}', Original Base: '${currentBaseName}')`);
		dispatch('confirm', { newName: nameToSend });
		closeModal();
	}

	function handleKeyDown(event) {
		if (event.key === 'Enter') {
			handleConfirm();
		} else if (event.key === 'Escape') {
			closeModal();
		}
	}

	function closeModal() {
		showModal = false;
		dispatch('close');
	}
</script>

{#if showModal}
	<div
		class="fixed inset-0 z-[120] flex items-center justify-center bg-black bg-opacity-60 backdrop-blur-sm"
		on:click|stopPropagation={closeModal}
		transition:fade={{ duration: 150 }}
	>
		<div
			class="bg-white dark:bg-surface-2 p-6 rounded-lg shadow-xl w-full max-w-md text-gray-900 dark:text-gray-200"
			on:click|stopPropagation
			role="dialog"
			aria-modal="true"
			aria-labelledby="rename-modal-title"
		>
			<h2 id="rename-modal-title" class="text-lg font-semibold mb-4">Rename Item</h2>

			<div class="mb-4">
				<label for="current-name-display" class="block text-sm font-medium text-gray-700 dark:text-gray-400"
					>Current name:</label
				>
                <!-- UPDATED: Use currentDisplayName -->
				<input
					id="current-name-display"
					type="text"
					readonly
					value={currentDisplayName}
					class="mt-1 block w-full px-3 py-2 bg-gray-100 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-md shadow-sm sm:text-sm text-gray-500 dark:text-gray-400 cursor-not-allowed"
				/>
			</div>

			<div class="mb-4">
				<label for="new-name" class="block text-sm font-medium text-gray-700 dark:text-gray-300"
					>New name:</label
				>
				<input
					bind:this={inputElement}
					bind:value={newNameBase}
					on:keydown={handleKeyDown}
					id="new-name"
					type="text"
					required
					class="mt-1 block w-full px-3 py-2 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500 sm:text-sm"
					aria-describedby="newNameHelp"
				/>
				{#if isStemInputMode}
					<p id="newNameHelp" class="mt-1 text-xs text-gray-500 dark:text-gray-400">
						{#if (itemType === 'doc' || itemType === 'table' || itemType === 'image') && currentExtension}
                            Enter the new file name.
						{:else if itemType === 'note' || itemType === 'transcript'}
                            Enter the new file name.
						{:else if itemType === 'media' || itemType === 'imported_transcript'}
							Enter the new file name.
                        {:else}
                            Enter just the file name. The original extension '<code>{currentExtension || '.ext'}</code>' will be used.
                        {/if}
						<!-- {#if itemType === 'media'}
							<br>This also renames the folder and primary transcript.
						{/if}
                        {#if itemType === 'doc' || itemType === 'table' || itemType === 'image' || itemType === 'imported_transcript'}
                             <br>This also renames the item's dedicated folder.
                        {/if} -->
					</p>
				{:else}
					<p id="newNameHelp" class="mt-1 text-xs text-gray-500 dark:text-gray-400">
						Enter the full new filename including the extension.
					</p>
				{/if}
			</div>

			{#if errorMessage}
				<p class="text-sm text-red-600 dark:text-red-400 mb-4" role="alert">{errorMessage}</p>
			{/if}

			<div class="flex justify-end space-x-3">
				<button
					type="button"
					class="px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-300 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-500 rounded-md shadow-sm hover:bg-gray-50 dark:hover:bg-gray-600 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-gray-500 dark:focus:ring-offset-gray-800"
					on:click={closeModal}
				>
					Cancel
				</button>
				<button
					type="button"
					class="px-4 py-2 text-sm font-medium text-white bg-blue-600 border border-transparent rounded-md shadow-sm hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 dark:focus:ring-offset-gray-800 disabled:opacity-50"
					on:click={handleConfirm}
					disabled={!newNameBase.trim() || !!errorMessage}
				>
					Rename
				</button>
			</div>
		</div>
	</div>
{/if}