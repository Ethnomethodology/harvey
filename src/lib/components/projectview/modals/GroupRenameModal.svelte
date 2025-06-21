<script>
    import { createEventDispatcher, onMount } from 'svelte';
    import { message } from '@tauri-apps/plugin-dialog';

    export let showModal = false;
    export let groupData = { id: null, name: '', description: '' }; // Incoming group data

    let currentName = '';
    let currentDescription = '';
    let isSaving = false;

    const dispatch = createEventDispatcher();

    $: if (showModal && groupData) {
        currentName = groupData.name || '';
        currentDescription = groupData.description || '';
    }

    function closeModal() {
        if (isSaving) return;
        showModal = false;
        // Don't reset currentName/currentDescription here, let the $: block handle it if groupData changes
        dispatch('close');
    }

    async function handleSave() {
        if (!currentName.trim()) {
            await message('Group name cannot be empty.', { title: 'Validation Error', type: 'error' });
            return;
        }
        if (!groupData || !groupData.id) {
            await message('Group context is missing. Cannot save.', { title: 'Error', type: 'error' });
            return;
        }

        isSaving = true;
        // Actual save logic (invoke backend) will be handled by parent listening to 'save' event for now,
        // or could be done here if invoke is directly used.
        // For this step, we just dispatch the event with the new data.
        dispatch('save', {
            groupId: groupData.id,
            newName: currentName.trim(),
            newDescription: currentDescription.trim() || null
        });
        // isSaving will be reset by parent after operation, or closeModal will be called.
        // For now, assume parent handles closing on successful save.
    }

    function handleKeydown(event) {
        if (event.key === 'Escape') {
            closeModal();
        } else if (event.key === 'Enter' && currentName.trim()) {
            event.preventDefault(); // Prevent form submission if inside a form
            handleSave();
        }
    }

    // Auto-focus the name input when modal becomes visible
    let nameInputRef;
    $: if (showModal && nameInputRef) {
        setTimeout(() => { // Ensure element is in DOM and rendered
            nameInputRef.focus();
            nameInputRef.select();
        }, 50);
    }

</script>

<svelte:window on:keydown={handleKeydown}/>

{#if showModal}
    <div class="fixed inset-0 z-[60] bg-gray-900 bg-opacity-50 dark:bg-opacity-75 flex items-center justify-center p-4" on:click|self={closeModal} role="dialog" aria-modal="true" aria-labelledby="rename-group-title">
        <div class="bg-white dark:bg-gray-800 p-5 rounded-lg shadow-xl w-full max-w-md" on:click|stopPropagation>
            <h2 id="rename-group-title" class="text-lg font-semibold text-gray-900 dark:text-white mb-4">Rename Group</h2>

            <div class="space-y-4">
                <div>
                    <label for="groupRenameName" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Group Name <span class="text-red-500">*</span></label>
                    <input bind:this={nameInputRef} type="text" id="groupRenameName" bind:value={currentName} placeholder="Enter group name"
                           class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:text-white sm:text-sm"
                           required
                           autocomplete="off"
                           autocorrect="off"
                           autocapitalize="off"
                           spellcheck="false" />
                </div>
                <div>
                    <label for="groupRenameDescription" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Description (Optional)</label>
                    <textarea id="groupRenameDescription" bind:value={currentDescription} rows="3" placeholder="Enter group description"
                              class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:text-white sm:text-sm"></textarea>
                </div>
            </div>

            <div class="mt-6 flex justify-end space-x-3">
                <button type="button" on:click={closeModal} disabled={isSaving}
                        class="px-4 py-2 text-sm font-medium text-gray-700 dark:text-gray-300 bg-white dark:bg-gray-700 border border-gray-300 dark:border-gray-500 rounded-md shadow-sm hover:bg-gray-50 dark:hover:bg-gray-600 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 disabled:opacity-50">
                    Cancel
                </button>
                <button type="button" on:click={handleSave} disabled={isSaving || !currentName.trim()}
                        class="px-4 py-2 text-sm font-medium text-white bg-blue-600 hover:bg-blue-700 border border-transparent rounded-md shadow-sm focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 disabled:opacity-50 disabled:bg-blue-400">
                    {#if isSaving}
                        Saving...
                    {:else}
                        Save Changes
                    {/if}
                </button>
            </div>
        </div>
    </div>
{/if}
