<script>
    import { createEventDispatcher, onMount } from 'svelte';
    import { confirm } from '@tauri-apps/plugin-dialog';
    import { Modal, Label, Input, Textarea, Button } from 'flowbite-svelte';

    export let showModal = false;
    export let group = null;

    const dispatch = createEventDispatcher();

    let name = '';
    let description = '';
    let isLoading = false;
    let errorMessage = '';

    onMount(() => {
        if (group) {
            name = group.name;
            description = group.description || '';
        }
    });

    function closeModal() {
        if (isLoading) return;
        dispatch('close');
    }

    async function handleSave() {
        if (!name.trim()) {
            errorMessage = 'Group name is required.';
            return;
        }
        errorMessage = '';
        isLoading = true;
        try {
            dispatch('save', {
                id: group.id,
                name: name.trim(),
                description: description.trim()
            });
        } catch (error) {
            errorMessage = `Failed to update group: ${error.message}`;
        } finally {
            isLoading = false;
        }
    }

    async function handleDelete() {
        const confirmed = await confirm(`Are you sure you want to delete the group "${group.name}"? All tags within this group will be deleted. Highlights will remain but will be untagged.`, {
            title: 'Confirm Group Deletion',
            type: 'warning',
        });

        if (confirmed) {
            isLoading = true;
            try {
                dispatch('delete', { id: group.id });
            } catch (error) {
                errorMessage = `Failed to delete group: ${error.message}`;
            } finally {
                isLoading = false;
            }
        }
    }
</script>

<Modal bind:open={showModal} size="sm" autoclose={false} outsideclose={true} class="w-full" on:close={closeModal}>
    <h2 class="text-lg font-semibold text-gray-800 dark:text-gray-100" slot="header">Edit Tag Group</h2>

    {#if errorMessage}
        <div class="mb-4 p-3 bg-red-100 dark:bg-red-700 border border-red-300 dark:border-red-600 text-red-700 dark:text-red-100 rounded-md text-sm">
            {errorMessage}
        </div>
    {/if}

    <form on:submit|preventDefault={handleSave} class="space-y-4">
        <div>
            <Label for="groupName" class="mb-1 text-sm font-medium text-gray-700 dark:text-gray-300">Group Name</Label>
            <Input
                type="text"
                id="groupName"
                bind:value={name}
                required
                autocomplete="off"
            />
        </div>

        <div>
            <Label for="groupDescription" class="mb-1 text-sm font-medium text-gray-700 dark:text-gray-300">Description (Optional)</Label>
            <Textarea
                id="groupDescription"
                bind:value={description}
                rows="3"
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
                Delete Group
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
                    disabled={isLoading || !name.trim()}
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
