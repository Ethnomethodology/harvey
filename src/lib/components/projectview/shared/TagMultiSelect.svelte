<script lang="ts">
    import MultiSelect from './MultiSelect.svelte';
    import { createEventDispatcher } from 'svelte';

    export let allTags: string[] = [];
    export let assignedTags: string[] = [];
    export let isEditable = true;

    const dispatch = createEventDispatcher();

    function handleUpdate(event) {
        dispatch('update', { tags: event.detail.options });
    }

    function handleCreate(event) {
        dispatch('createtag', { tag: event.detail.option });
    }
</script>

<MultiSelect
    itemType="tag"
    allOptions={allTags}
    assignedOptions={assignedTags}
    {isEditable}
    placeholder="No tags assigned."
    on:update={handleUpdate}
    on:create={handleCreate}
/>