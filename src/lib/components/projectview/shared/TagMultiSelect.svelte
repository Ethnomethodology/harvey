<script lang="ts">
  import MultiSelect from './MultiSelect.svelte';
  import { createEventDispatcher } from 'svelte';
  import { tagStore } from '$lib/stores/tagStore.svelte.js';

  let {
    assignedTags = [],
    isEditable = true
  } = $props();

  const dispatch = createEventDispatcher();

  let groupedTags = $derived(
    tagStore.allTagGroups
      .map((group) => ({
        name: group.name,
        options: tagStore.allTags
          .filter((tag) => tag.tag_group_id === group.id)
          .map((tag) => tag.name)
      }))
      .filter((group) => group.options.length > 0)
  );

  let allTagNames = $derived(tagStore.allTags.map((t) => t.name));

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
