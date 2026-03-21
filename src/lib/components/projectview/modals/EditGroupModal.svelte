<script>
    import { createEventDispatcher, onMount } from 'svelte';
    import { invoke } from '@tauri-apps/api/core';
    import { message, confirm } from '@tauri-apps/plugin-dialog';
    import { Modal, Label, Input, Textarea, Button } from 'flowbite-svelte';
    import { PencilLine, Trash2 } from '@lucide/svelte';

    export let showModal = false;
    export let groupData = null; // Expected: { id, project_id, name, description }

    let currentName = '';
    let currentDescription = '';
    let isLoading = false;
    let errorMessage = '';

    const dispatch = createEventDispatcher();

    // onMount is not strictly needed for this $: block, but doesn't harm
    onMount(() => {
        // Initialization logic handled by the reactive block below
    });

    // Reactive block to update form fields when groupData changes or modal becomes visible
    $: if (groupData && showModal) {
        currentName = groupData.name || '';
        currentDescription = groupData.description || '';
        errorMessage = ''; // Clear previous errors when modal reopens or groupData changes
    } else if (!showModal) {
        // Optional: Clear fields when modal is not shown to prevent stale data if groupData is nulled out later
        // currentName = '';
        // currentDescription = '';
        // errorMessage = '';
    }


    async function handleSave() {
        if (!groupData || !groupData.id || !groupData.project_id) {
            errorMessage = "Group data is incomplete. Cannot save.";
            return;
        }
        if (!currentName.trim()) {
            errorMessage = "Group name cannot be empty.";
            return;
        }

        isLoading = true;
        errorMessage = '';

        try {
            const updatedGroup = await invoke('update_group_details', {
                projectId: groupData.project_id,
                groupId: groupData.id,
                name: currentName.trim(),
                description: currentDescription.trim() === '' ? null : currentDescription.trim()
            });
            dispatch('groupUpdated', updatedGroup);
            closeModal();
        } catch (err) {
            console.error("Error updating group:", err);
            errorMessage = typeof err === 'string' ? err : "Failed to update group.";
            // Specific check for unique constraint error from backend
            if (typeof err === 'string' && err.toLowerCase().includes("a group with this name already exists")) {
                errorMessage = err; // Use the backend's specific message
            }
        } finally {
            isLoading = false;
        }
    }

    async function handleDelete() {
        if (!groupData || !groupData.id || !groupData.project_id) return;

        const confirmed = await confirm(
            `Are you sure you want to delete the group "${groupData.name}"? This action cannot be undone.`,
            { title: 'Confirm Delete Group', type: 'warning' }
        );

        if (!confirmed) return;

        isLoading = true;
        errorMessage = '';

        try {
            await invoke('delete_project_group', {
                projectId: groupData.project_id,
                groupId: groupData.id
            });
            dispatch('groupDeleted', groupData.id);
            closeModal();
        } catch (err) {
            console.error("Error deleting group:", err);
            errorMessage = typeof err === 'string' ? err : "Failed to delete group.";
        } finally {
            isLoading = false;
        }
    }

    function closeModal() {
        // Fields are reset by the $: block when showModal becomes false if groupData is also cleared by parent,
        // or when new groupData is passed upon reopening.
        dispatch('close');
    }
</script>

<Modal
    bind:open={showModal}
    size="sm"
    autoclose={false}
    outsideclose={true}
    class="w-full"
    backdropClass="fixed inset-0 z-[10000] bg-black/60 backdrop-blur-sm"
    dialogClass="fixed top-0 start-0 end-0 h-modal md:inset-0 md:h-full z-[10001] flex"
    bodyClass="p-6 space-y-4 bg-white dark:bg-gray-900"
    headerClass="px-6 py-4 flex items-center justify-between border-b dark:border-gray-700 bg-gray-50/50"
    footerClass="px-6 py-4 flex items-center justify-end space-x-3 rtl:space-x-reverse border-t dark:border-gray-700 bg-gray-50/80 backdrop-blur"
    on:close={closeModal}
>
    <div slot="header" class="flex items-center gap-2">
        <PencilLine class="w-5 h-5 text-gray-500" />
        <h3 class="text-lg font-semibold text-gray-900 dark:text-white">
            Edit Group
        </h3>
    </div>

    {#if errorMessage}
        <div class="p-3 bg-red-100 dark:bg-red-700 border border-red-300 dark:border-red-600 text-red-700 dark:text-red-100 rounded-md text-sm">
            {errorMessage}
        </div>
    {/if}

    <div class="space-y-4">
        <div>
            <Label for="editGroupName" class="mb-1 text-sm font-medium text-gray-700 dark:text-gray-300">Group Name</Label>
            <Input
                type="text"
                id="editGroupName"
                bind:value={currentName}
                required
                autocomplete="off" autocorrect="off" autocapitalize="off" spellcheck="false"
            />
        </div>

        <div>
            <Label for="editGroupDescription" class="mb-1 text-sm font-medium text-gray-700 dark:text-gray-300">Description (Optional)</Label>
            <Textarea
                id="editGroupDescription"
                bind:value={currentDescription}
                rows="3"
                autocomplete="off" autocorrect="off" autocapitalize="off" spellcheck="false"
            ></Textarea>
        </div>
    </div>

    <svelte:fragment slot="footer">
        <div class="flex-grow">
            <Button
                color="red"
                outline
                on:click={handleDelete}
                disabled={isLoading}
                title="Delete this group"
                class="gap-2"
            >
                <Trash2 class="w-4 h-4" />
                Delete Group
            </Button>
        </div>
        <Button
            color="alternative"
            on:click={closeModal}
            disabled={isLoading}
            title="Cancel editing"
        >
            Cancel
        </Button>
        <Button
            color="blue"
            on:click={handleSave}
            disabled={isLoading || !currentName.trim()}
            title="Save changes"
        >
            {#if isLoading}
                Saving...
            {:else}
                Save Changes
            {/if}
        </Button>
    </svelte:fragment>
</Modal>
