<script>
    import { createEventDispatcher, onMount } from 'svelte';
    import { confirm } from '@tauri-apps/plugin-dialog';
    import { Modal, Label, Input, Textarea, Button } from 'flowbite-svelte';
    import { Tags, Trash2 } from 'lucide-svelte';

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
    footerClass="px-6 py-4 flex items-center justify-between border-t dark:border-gray-700 bg-gray-50/80 backdrop-blur"
    on:close={closeModal}
>
    <div slot="header" class="flex items-center gap-2">
        <Tags class="w-5 h-5 text-gray-500" />
        <h3 class="text-lg font-semibold text-gray-900 dark:text-white">
            Edit Tag Group
        </h3>
    </div>

    {#if errorMessage}
        <div class="p-3 bg-red-100 dark:bg-red-700 border border-red-300 dark:border-red-600 text-red-700 dark:text-red-100 rounded-md text-sm">
            {errorMessage}
        </div>
    {/if}

    <div class="space-y-4">
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
    </div>

    <svelte:fragment slot="footer">
        <Button
            color="red"
            outline
            on:click={handleDelete}
            disabled={isLoading}
            title="Delete this tag group"
            class="px-3"
        >
            <Trash2 class="w-4 h-4 mr-2" />
            Delete
        </Button>
        <div class="flex space-x-3">
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
                disabled={isLoading || !name.trim()}
                title="Save changes"
            >
                {#if isLoading}
                    Saving...
                {:else}
                    Save Changes
                {/if}
            </Button>
        </div>
    </svelte:fragment>
</Modal>
