<script>
    import { createEventDispatcher, onMount } from 'svelte';
    import { invoke } from '@tauri-apps/api/core';
    import { message } from '@tauri-apps/plugin-dialog';
    import { Modal, Label, Input, Textarea, Button } from 'flowbite-svelte';
    import { updateProjectGroupsList } from '$lib/stores/projectStore.js';

    export let showModal = false;
    export let projectUuid = null; // Passed in from parent
    export let fileToAdd = null;   // Passed in from parent (contains relativePath, name)

    let groupName = '';
    let groupDescription = '';
    let isSaving = false;

    const dispatch = createEventDispatcher();

    onMount(() => {
    });

    function closeModal() {
        if (isSaving) return;
        showModal = false;
        groupName = '';
        groupDescription = '';
        dispatch('close');
    }

    async function handleSave() {
        console.log('[CreateGroupModal] handleSave triggered. fileToAdd object:', fileToAdd);
        if (fileToAdd) {
            console.log('[CreateGroupModal] fileToAdd.name:', fileToAdd.name);
            console.log('[CreateGroupModal] fileToAdd.path:', fileToAdd.path);
            console.log('[CreateGroupModal] fileToAdd.relativePath (camelCase):', fileToAdd.relativePath);
            console.log('[CreateGroupModal] fileToAdd.relative_path (snake_case):', fileToAdd.relative_path);
            console.log('[CreateGroupModal] fileToAdd.asset_relative_path:', fileToAdd.asset_relative_path);
            console.log('[CreateGroupModal] fileToAdd keys:', Object.keys(fileToAdd));
        } else {
            console.log('[CreateGroupModal] fileToAdd is null or undefined.');
        }
        if (!groupName.trim()) {
            await message('Group name cannot be empty.', { title: 'Validation Error', type: 'error' });
            return;
        }
        if (!projectUuid) {
            await message('Project context is missing.', { title: 'Error', type: 'error' });
            return;
        }

        isSaving = true;
        try {
            const newGroup = await invoke('create_new_group', {
                projectId: projectUuid,
                name: groupName.trim(),
                description: groupDescription.trim() || null,
                fileAssetRelativePath: fileToAdd ? (fileToAdd.relative_path || fileToAdd.relativePath) : null,
            });

            // newGroup is the GroupData returned from the backend command
            if (newGroup) { // If group creation was successful (backend returns GroupData)
                if (projectUuid) { // Ensure projectUuid is valid before updating list
                    await updateProjectGroupsList(projectUuid);
                }
                if (fileToAdd && (fileToAdd.relative_path || fileToAdd.relativePath)) {
                    // Even if backend handles association, dispatch this event for UI consistency
                    // if fileToAdd was intended. The backend log will show if association failed.
                    dispatch('groupCreatedAndFileAdded', { group: newGroup, file: fileToAdd });
                } else {
                    dispatch('groupCreated', { group: newGroup });
                }
            }
            // If newGroup is null or command failed, an error would have been thrown by invoke
            closeModal();
        } catch (err) {
            console.error('Error creating group:', err); // Error might include file association issues now
            await message(`Failed to create group: ${err}`, { title: 'Error', type: 'error' });
        } finally {
            isSaving = false;
        }
    }

    // Handle Escape key to close modal
    function handleKeydown(event) {
        if (event.key === 'Escape') {
            closeModal();
        }
    }
</script>

<svelte:window on:keydown={handleKeydown}/>

<Modal bind:open={showModal} size="sm" autoclose={false} outsideclose={true} class="w-full z-[60]" on:close={closeModal}>
    <h2 class="text-lg font-semibold text-gray-900 dark:text-white mb-4" slot="header">Create New Group</h2>

    <div class="space-y-4">
        <div>
            <Label for="groupName" class="mb-1 text-sm font-medium text-gray-700 dark:text-gray-300">Group Name <span class="text-red-500">*</span></Label>
            <Input type="text" id="groupName" bind:value={groupName} placeholder="Enter group name"
                   required
                   autocomplete="off"
                   autocorrect="off"
                   autocapitalize="off"
                   spellcheck="false" />
        </div>
        <div>
            <Label for="groupDescription" class="mb-1 text-sm font-medium text-gray-700 dark:text-gray-300">Description (Optional)</Label>
            <Textarea id="groupDescription" bind:value={groupDescription} rows="3" placeholder="Enter group description"
                      autocomplete="off"
                      autocorrect="off"></Textarea>
        </div>
    </div>

    <svelte:fragment slot="footer">
        <div class="flex justify-end space-x-3 w-full">
            <Button color="alternative" on:click={closeModal} disabled={isSaving}>
                Cancel
            </Button>
            <Button color="blue" on:click={handleSave} disabled={isSaving || !groupName.trim()}>
                {#if isSaving}
                    Saving...
                {:else}
                    Save Group
                {/if}
            </Button>
        </div>
    </svelte:fragment>
</Modal>
