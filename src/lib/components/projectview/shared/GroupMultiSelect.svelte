<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';

  // Define GroupData directly in this file
  interface GroupData {
    id: string;
    project_id: string;
    name: string;
    description?: string | null; // Optional description
  }

  export let fileAssetRelativePath: string;
  export let projectId: string;
  export let allProjectGroups: GroupData[] = [];
  export let initiallyAssignedGroups: GroupData[] = [];
  export let isEditable = true; // New prop

  let assignedGroups: GroupData[] = [];
  let availableGroupsToAssign: GroupData[] = [];
  let showDropdown = false;
  let searchTerm = '';
  let rootElement: HTMLElement;
  let dropdownX = 0;
  let dropdownY = 0;
  let dropdownWidth = 0;

  const dispatch = createEventDispatcher();

  // Reactive statements to update internal state when props change
  $: {
    assignedGroups = [...(initiallyAssignedGroups || [])];
    updateAvailableGroups();
  }

  function updateAvailableGroups() {
    const assignedGroupIds = new Set(assignedGroups.map((g) => g.id));
    availableGroupsToAssign = (allProjectGroups || []).filter((g) => !assignedGroupIds.has(g.id));
  }

  async function removeGroup(group: GroupData) {
    if (!isEditable) return;
    if (!projectId || !fileAssetRelativePath || !group.id) {
      console.error('Missing data for removing group from file', {
        projectId,
        fileAssetRelativePath,
        groupId: group.id
      });
      dispatch('error', 'Failed to remove group: Missing critical data.');
      return;
    }
    try {
      await invoke('remove_file_from_group', {
        projectId: projectId,
        groupId: group.id,
        fileAssetRelativePath: fileAssetRelativePath
      });
      assignedGroups = assignedGroups.filter((g) => g.id !== group.id);
      updateAvailableGroups();
      dispatch('groupsUpdated', { action: 'removed', group });
    } catch (err) {
      console.error('Error removing file from group:', err);
      dispatch('error', `Failed to remove group: ${err}`);
    }
  }

  async function addGroup(group: GroupData) {
    if (!isEditable) return;
    if (!projectId || !fileAssetRelativePath || !group.id) {
      console.error('Missing data for adding group to file', {
        projectId,
        fileAssetRelativePath,
        groupId: group.id
      });
      dispatch('error', 'Failed to add group: Missing critical data.');
      return;
    }
    try {
      await invoke('add_file_to_existing_group', {
        projectId: projectId,
        groupId: group.id,
        fileAssetRelativePath: fileAssetRelativePath
      });
      assignedGroups = [...assignedGroups, group];
      updateAvailableGroups();
      searchTerm = '';
      dispatch('groupsUpdated', { action: 'added', group });
    } catch (err) {
      console.error('Error adding file to group:', err);
      dispatch('error', `Failed to add group: ${err}`);
    }
  }

  function handleCreateNewGroup() {
    if (!isEditable) return;
    dispatch('createNewGroup');
  }

  $: filteredAvailableGroups = searchTerm
    ? availableGroupsToAssign.filter((group) =>
        group.name.toLowerCase().includes(searchTerm.toLowerCase())
      )
    : availableGroupsToAssign;

  function toggleDropdown() {
    showDropdown = !showDropdown;
    if (showDropdown) {
      // Reset search term when opening dropdown
      searchTerm = '';
      updateDropdownPosition();
    }
  }

  function updateDropdownPosition() {
    if (rootElement) {
      const rect = rootElement.getBoundingClientRect();
      const dropdownHeight = 240; // max-h-60 = 240px
      const margin = 5;

      // Document-relative coordinates (allows for absolute positioning attached to body)
      const scrollX = window.scrollX || window.pageXOffset;
      const scrollY = window.scrollY || window.pageYOffset;

      // Default position: below
      dropdownX = rect.left + scrollX;
      dropdownY = rect.bottom + scrollY + margin;
      dropdownWidth = rect.width;

      // Flip logic: if it overflows viewport bottom, check if there's more space above
      if (rect.bottom + dropdownHeight > window.innerHeight) {
        const spaceAbove = rect.top;
        const spaceBelow = window.innerHeight - rect.bottom;

        if (spaceAbove > spaceBelow && spaceAbove > 100) {
          // Position above
          dropdownY = rect.top + scrollY - Math.min(dropdownHeight, spaceAbove - margin) - margin;
        }
      }

      // Horizontal shift to stay within viewport (still relative to document width)
      if (rect.left + dropdownWidth > window.innerWidth) {
        dropdownX = Math.max(margin, window.innerWidth - dropdownWidth - margin) + scrollX;
      }
    }
  }

  function portal(node) {
    document.body.appendChild(node);
    return {
      destroy() {
        if (node.parentNode) {
          node.parentNode.removeChild(node);
        }
      }
    };
  }

  import { onMount, onDestroy } from 'svelte';
  let resizeObserver: ResizeObserver | null = null;

  onMount(() => {
    if (typeof ResizeObserver !== 'undefined' && rootElement) {
      resizeObserver = new ResizeObserver(() => {
        if (showDropdown) updateDropdownPosition();
      });
      resizeObserver.observe(rootElement);
    }
  });

  onDestroy(() => {
    if (resizeObserver) resizeObserver.disconnect();
  });

  // Reactive trigger for position update when showDropdown becomes true
  $: if (showDropdown && rootElement) {
    setTimeout(updateDropdownPosition, 0);
  }
</script>

<div class="relative" bind:this={rootElement}>
  <!-- Assigned Groups Tags -->
  <div
    class="flex flex-wrap gap-1 mb-2 p-1 border border-gray-300 dark:border-gray-600 rounded-md min-h-[30px]"
  >
    {#if assignedGroups.length === 0}
      <span class="text-xs text-gray-500 dark:text-gray-400 px-2 py-1">No groups assigned.</span>
    {/if}
    {#each assignedGroups as group (group.id)}
      <span
        class="flex items-center bg-blue-100 dark:bg-blue-700 text-blue-800 dark:text-blue-200 text-xs font-medium px-2 py-0.5 rounded-full"
      >
        {group.name}
        {#if isEditable}
          <button
            on:click|stopPropagation={() => removeGroup(group)}
            class="ml-1.5 text-blue-600 dark:text-blue-300 hover:text-blue-800 dark:hover:text-blue-100"
            aria-label="Remove {group.name}"
          >
            <svg
              xmlns="http://www.w3.org/2000/svg"
              viewBox="0 0 20 20"
              fill="currentColor"
              class="h-3 w-3"
            >
              <path
                fill-rule="evenodd"
                d="M4.293 4.293a1 1 0 011.414 0L10 8.586l4.293-4.293a1 1 0 111.414 1.414L11.414 10l4.293 4.293a1 1 0 01-1.414 1.414L10 11.414l-4.293 4.293a1 1 0 01-1.414-1.414L8.586 10 4.293 5.707a1 1 0 010-1.414z"
                clip-rule="evenodd"
              />
            </svg>
          </button>
        {/if}
      </span>
    {/each}
    <!-- Dropdown Toggle Area (rest of the clickable space if no groups) -->
    <div
      class="flex-grow min-w-[50px]"
      on:click={() => isEditable && toggleDropdown()}
      class:cursor-not-allowed={!isEditable}
      class:opacity-75={!isEditable}
      role={isEditable ? 'button' : 'presentation'}
      tabindex={isEditable ? 0 : -1}
      on:keydown={(e) => {
        if (isEditable && (e.key === 'Enter' || e.key === ' ')) toggleDropdown();
      }}
    >
      {#if assignedGroups.length > 0 && isEditable}
        <button
          type="button"
          class="w-full text-left px-2 py-1 text-xs text-gray-500 dark:text-gray-400 hover:bg-gray-50 dark:hover:bg-gray-700 focus:outline-none"
          aria-haspopup="true"
          aria-expanded={showDropdown}
          disabled={!isEditable}
        >
          Add to group...
        </button>
      {:else if assignedGroups.length === 0 && isEditable}
        <!-- If no groups assigned but editable, clicking the empty space should still toggle dropdown -->
        <div class="w-full h-full" />
      {/if}
    </div>
  </div>

  <!-- Dropdown -->
  {#if showDropdown}
    <div
      use:portal
      class="absolute z-[999999] bg-white dark:bg-gray-900 border border-gray-300 dark:border-gray-700 rounded-md shadow-lg max-h-60 overflow-y-auto group-multi-select-dropdown"
      style="top: {dropdownY}px; left: {dropdownX}px; width: {dropdownWidth}px;"
    >
      <div class="p-2">
        <input
          type="text"
          bind:value={searchTerm}
          placeholder="Search or add new..."
          class="w-full px-2 py-1.5 text-xs border border-gray-300 dark:border-gray-700 rounded-md dark:bg-gray-800 dark:text-white focus:ring-blue-500 focus:border-blue-500"
          autocomplete="off"
          autocorrect="off"
        />
      </div>
      <ul>
        {#each filteredAvailableGroups as group (group.id)}
          <li
            on:click|stopPropagation={() => addGroup(group)}
            class="px-3 py-1.5 text-xs hover:bg-gray-100 dark:hover:bg-gray-800 cursor-pointer text-gray-700 dark:text-gray-200"
          >
            {group.name}
          </li>
        {/each}

        {#if searchTerm && !filteredAvailableGroups.find((g) => g.name.toLowerCase() === searchTerm.toLowerCase())}
          <!-- Show create new if search term does not exactly match an existing available group -->
          <!-- This part might be too simplistic, a dedicated "Create Group" button is better -->
        {/if}
        {#if isEditable}
          <li
            on:click|stopPropagation={handleCreateNewGroup}
            class="px-3 py-1.5 text-xs text-blue-600 dark:text-blue-400 hover:bg-gray-100 dark:hover:bg-gray-800 cursor-pointer border-t border-gray-200 dark:border-gray-700"
          >
            + Create new group {searchTerm ? `"${searchTerm}"` : ''}
          </li>
        {/if}
      </ul>
    </div>
  {/if}
</div>

<svelte:window
  on:click={(event) => {
    const target = event.target as HTMLElement;
    if (!target.closest('.relative') && !target.closest('.group-multi-select-dropdown')) {
      // Clicked outside the component and its dropdown
      showDropdown = false;
    }
  }}
  on:resize={updateDropdownPosition}
  on:scroll|capture={updateDropdownPosition}
/>

<style>
  /* Ensure dropdown appears above other elements */
  .relative {
    position: relative;
  }
  .absolute {
    position: absolute;
  }
</style>
