<script>
    import { createEventDispatcher, onMount } from 'svelte';
    import { message } from '@tauri-apps/plugin-dialog';
    import { Modal, Label, Input, Textarea, Button } from 'flowbite-svelte';
    import { PencilLine } from '@lucide/svelte';

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
        if (event.key === 'Enter' && currentName.trim()) {
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
            Rename Group
        </h3>
    </div>

    <div class="space-y-4">
        <div>
            <Label for="groupRenameName" class="mb-1 text-sm font-medium text-gray-700 dark:text-gray-300">Group Name <span class="text-red-500">*</span></Label>
            <Input
                bind:this={nameInputRef}
                type="text"
                id="groupRenameName"
                bind:value={currentName}
                placeholder="Enter group name"
                required
                autocomplete="off"
                autocorrect="off"
                autocapitalize="off"
                spellcheck="false"
                on:keydown={handleKeydown}
            />
        </div>
        <div>
            <Label for="groupRenameDescription" class="mb-1 text-sm font-medium text-gray-700 dark:text-gray-300">Description (Optional)</Label>
            <Textarea id="groupRenameDescription" bind:value={currentDescription} rows="3" placeholder="Enter group description"
                      autocomplete="off"
                      autocorrect="off"
                      autocapitalize="off"
                      spellcheck="false"></Textarea>
        </div>
    </div>

    <svelte:fragment slot="footer">
        <Button color="alternative" on:click={closeModal} disabled={isSaving} title="Cancel renaming">
            Cancel
        </Button>
        <Button color="blue" on:click={handleSave} disabled={isSaving || !currentName.trim()} title="Save changes">
            {#if isSaving}
                Saving...
            {:else}
                Save Changes
            {/if}
        </Button>
    </svelte:fragment>
</Modal>
