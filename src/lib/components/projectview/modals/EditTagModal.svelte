<script>
    import { createEventDispatcher, onMount } from 'svelte';
    import { confirm } from '@tauri-apps/plugin-dialog';
    import { Modal, Label, Input, Textarea, Button } from 'flowbite-svelte';

    export let showModal = false;
    export let tag = null;

    const dispatch = createEventDispatcher();

    let currentName = '';
    let currentDescription = '';
    let isLoading = false;
    let errorMessage = '';

    onMount(() => {
        if (tag) {
            currentName = tag.name;
            currentDescription = tag.description || '';
        }
    });

    function closeModal() {
        if (isLoading) return;
        dispatch('close');
    }

    async function handleSave() {
        if (!currentName.trim()) {
            errorMessage = 'Tag name cannot be empty.';
            return;
        }
        errorMessage = '';
        isLoading = true;
        try {
            dispatch('save', {
                id: tag.id,
                name: currentName,
                description: currentDescription
            });
        } catch (error) {
            errorMessage = `Failed to save tag: ${error.message}`;
        } finally {
            isLoading = false;
        }
    }

    async function handleDelete() {
        const confirmed = await confirm(`Are you sure you want to delete the tag "${tag.name}"? This will remove the tag from all associated highlights and cannot be undone.`, {
            title: 'Confirm Deletion',
            type: 'warning',
        });

        if (confirmed) {
            isLoading = true;
            try {
                dispatch('delete', { id: tag.id });
            } catch (error) {
                errorMessage = `Failed to delete tag: ${error.message}`;
            } finally {
                isLoading = false;
            }
        }
    }
</script>

<Modal bind:open={showModal} size="sm" autoclose={false} outsideclose={true} class="w-full" on:close={closeModal}>
    <h2 class="text-lg font-semibold text-gray-800 dark:text-gray-100" slot="header">Edit Tag</h2>

    {#if errorMessage}
        <div class="mb-4 p-3 bg-red-100 dark:bg-red-700 border border-red-300 dark:border-red-600 text-red-700 dark:text-red-100 rounded-md text-sm">
            {errorMessage}
        </div>
    {/if}

    <form on:submit|preventDefault={handleSave} class="space-y-4">
        <div>
            <Label for="editTagName" class="mb-1 text-sm font-medium text-gray-700 dark:text-gray-300">Tag Name</Label>
            <Input
                type="text"
                id="editTagName"
                bind:value={currentName}
                required
                autocomplete="off" autocorrect="off" autocapitalize="off" spellcheck="false"
            />
        </div>

        <div>
            <Label for="editTagDescription" class="mb-1 text-sm font-medium text-gray-700 dark:text-gray-300">Description (Optional)</Label>
            <Textarea
                id="editTagDescription"
                bind:value={currentDescription}
                rows="3"
                autocomplete="off" autocorrect="off" autocapitalize="off" spellcheck="false"
            ></Textarea>
        </div>
    </form>

    <svelte:fragment slot="footer">
        <div class="flex justify-between items-center w-full">
            <Button
                color="red"
                outline
                on:click={handleDelete}
                disabled={isLoading}
            >
                Delete
            </Button>
            <div class="flex space-x-3">
                <Button
                    color="alternative"
                    on:click={closeModal}
                    disabled={isLoading}
                >
                    Cancel
                </Button>
                <Button
                    color="blue"
                    on:click={handleSave}
                    disabled={isLoading || !currentName.trim()}
                >
                    {#if isLoading}
                        Saving...
                    {:else}
                        Save Changes
                    {/if}
                </Button>
            </div>
        </div>
    </svelte:fragment>
</Modal>
