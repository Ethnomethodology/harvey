<script lang="ts">
    import MultiSelect from './MultiSelect.svelte';
    import { createEventDispatcher } from 'svelte';
    import { allTags, allTagGroups } from '$lib/stores/tagStore.js';
 
    export let assignedTags: string[] = [];
    export let isEditable = true;
 
    const dispatch = createEventDispatcher();

    $: groupedTags = $allTagGroups.map(group => ({
        name: group.name,
        options: $allTags
            .filter(tag => tag.tag_group_id === group.id)
            .map(tag => tag.name)
    })).filter(group => group.options.length > 0);

    $: allTagNames = $allTags.map(t => t.name);
 
    function handleUpdate(event) {
        dispatch('update', { tags: event.detail.options });
    }
 
    function handleCreate(event) {
        dispatch('createtag', { tag: event.detail.option });
    }
</script>

<MultiSelect
    itemType="tag"
    allOptions={allTagNames}
    groupedOptions={groupedTags}
    assignedOptions={assignedTags}
    {isEditable}
    placeholder="No tags assigned."
    on:update={handleUpdate}
    on:create={handleCreate}
/>