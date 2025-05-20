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

	$: isExtensionFixed = itemType === 'media' || itemType === 'note' || itemType === 'transcript' || itemType === 'doc';

	let currentBaseName = '';
	let currentExtension = '';
    // --- NEW: Reactive variable for display name ---
    let currentDisplayName = '';

	function updateNameParts() {
		if (currentName) {
			const lastDotIndex = currentName.lastIndexOf('.');
			if (lastDotIndex > 0 && lastDotIndex < currentName.length - 1) {
				// Found a likely extension
				currentBaseName = currentName.substring(0, lastDotIndex);
				currentExtension = currentName.substring(lastDotIndex);
                // --- Display logic based on type ---
                if (itemType === 'media' || itemType === 'doc') {
                    currentDisplayName = currentBaseName; // Show only base for media and doc
                } else {
                    currentDisplayName = currentName; // Show full name for others by default
                }
			} else {
				// No extension found or dot is at the beginning
				currentBaseName = currentName;
				currentExtension = '';
				currentDisplayName = currentName; // Show the full name if no extension
                if (isExtensionFixed && itemType !== 'media') { // Don't warn for media stem identifier
                     console.warn(`[Rename Modal] Could not extract extension from '${currentName}' for fixed type '${itemType}'.`);
                }
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

	function validateName(nameToCheck) {
		const base = nameToCheck.trim();
		if (!base) {
			return 'Name cannot be empty.';
		}
		if (/[<>:"/\\|?*]/.test(base)) {
			return 'Name contains invalid characters (< > : " / \\ | ? *).';
		}
		if (base.startsWith('.')) {
			return 'Name cannot start with a dot.';
		}
		// Allow dots in transcript base names, but not others where extension is fixed
		if (isExtensionFixed && base.includes('.') && itemType !== 'transcript') {
			return `Base name cannot contain dots for ${itemType} items (extension '${currentExtension || '(.ext)'}' is added automatically).`;
		}
        if (!isExtensionFixed && !base.includes('.')) {
             return 'Filename must include an extension.';
        }
		return '';
	}

	function handleConfirm() {
		errorMessage = validateName(newNameBase);
		if (errorMessage) {
			return;
		}

        // Construct final name, ensuring correct extension for fixed types
        let finalNewName = '';
        if(isExtensionFixed) {
            // Determine the *correct* expected extension based on type
            let expectedExtension = '';
            if (itemType === 'doc' || itemType === 'transcript' || itemType === 'note') {
                 // Assuming notes are also JSON for now based on previous context
                 expectedExtension = '.json';
            } else if (itemType === 'media') {
                // Media uses original extension derived earlier, or might be complex if format changed
                 expectedExtension = currentExtension; // Use the derived extension
            }
            // Handle cases where original extension might be missing but type is fixed
            if (!expectedExtension && (itemType === 'doc' || itemType === 'transcript' || itemType === 'note')) {
                console.warn(`[Rename Modal] Missing original extension for fixed type '${itemType}', defaulting to '.json'`);
                expectedExtension = '.json';
            } else if (!expectedExtension && itemType === 'media') {
                 console.error(`[Rename Modal] Cannot determine original extension for media type rename. Aborting.`);
                 errorMessage = 'Cannot determine original file type for media.';
                 return;
            }
             finalNewName = `${newNameBase.trim()}${expectedExtension}`;
        } else {
            finalNewName = newNameBase.trim(); // User provided full name with extension
        }


		if (finalNewName === currentName) {
			errorMessage = 'New name is the same as the current name.';
			return;
		}

        // For media, still send only the base name (stem)
		const nameToSend = itemType === 'media' ? newNameBase.trim() : finalNewName;

		console.log(`[Rename Modal] Dispatching confirm. Item Type: '${itemType}', Name to Send: '${nameToSend}' (Final constructed: '${finalNewName}', Original: '${currentName}')`);
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
		class="fixed inset-0 z-50 flex items-center justify-center bg-black bg-opacity-60 backdrop-blur-sm"
		on:click|stopPropagation={closeModal}
		transition:fade={{ duration: 150 }}
	>
		<div
			class="bg-white dark:bg-gray-800 p-6 rounded-lg shadow-xl w-full max-w-md text-gray-900 dark:text-gray-200"
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
                <!-- UPDATED: Removed "(without extension)" text -->
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
				{#if isExtensionFixed}
					<p id="newNameHelp" class="mt-1 text-xs text-gray-500 dark:text-gray-400">
						{#if currentExtension}
                            The extension '<code>{currentExtension}</code>' will be automatically appended.
                        {:else if itemType === 'media'}
                            Renaming media source. Extension determined automatically.
                        {:else if itemType === 'doc' || itemType === 'transcript' || itemType === 'note'}
                            The extension '<code>.json</code>' will be automatically appended.
                        {:else}
                            Enter the base name. Extension will be added.
                        {/if}
						{#if itemType === 'media'}
							<br>This also renames the folder and primary transcript.
						{/if}
                        {#if itemType === 'transcript'}
                             <br>Renaming primary transcript may break auto-loading.
                        {/if}
                         {#if itemType === 'doc'}
                             <br>Documents are saved with a '.json' extension.
                        {/if}
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