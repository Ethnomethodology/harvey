<script>
    import { createEventDispatcher } from 'svelte';
    import { Modal, Label, Input, Textarea, Button } from 'flowbite-svelte';

    export let showModal = false;

    const dispatch = createEventDispatcher();

    let name = '';
    let description = '';
    let isLoading = false;
    let errorMessage = '';

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
                name: name.trim(),
                description: description.trim()
            });
        } catch (error) {
            errorMessage = `Failed to create group: ${error.message}`;
        } finally {
            isLoading = false;
        }
    }
</script>

<Modal bind:open={showModal} size="sm" autoclose={false} outsideclose={true} class="w-full" on:close={closeModal}>
    <h2 class="text-lg font-semibold text-gray-800 dark:text-gray-100" slot="header">Add Tag Group</h2>

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
                placeholder="e.g., Themes, Speakers"
            />
        </div>

        <div>
            <Label for="groupDescription" class="mb-1 text-sm font-medium text-gray-700 dark:text-gray-300">Description (Optional)</Label>
            <Textarea
                id="groupDescription"
                bind:value={description}
                rows="3"
                placeholder="Describe what this group represents..."
            ></Textarea>
        </div>
    </form>

    <svelte:fragment slot="footer">
        <div class="flex justify-end space-x-3 w-full">
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
                    Creating...
                {:else}
                    Create Group
                {/if}
            </Button>
        </div>
    </svelte:fragment>
</Modal>
