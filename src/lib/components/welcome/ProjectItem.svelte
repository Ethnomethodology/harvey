<!-- src/lib/components/welcome/ProjectItem.svelte -->
<script>
  import { fly } from 'svelte/transition';
  import { createEventDispatcher } from 'svelte';
  export let project;
  export let openMenuProjectPath = null;

  const dispatch = createEventDispatcher();

  function openRecent() {
    dispatch('openRecent', project);
  }

  function toggleMenu(event) {
    event.stopPropagation();
    dispatch('toggleMenu', project.path);
  }

  function onMenuAction(action) {
    dispatch('menuAction', { action, project });
  }
</script>

<div class="relative group menu-container">
  <li
    class="p-3 bg-gray-50 rounded-lg border border-gray-200 hover:shadow-md hover:bg-gray-100 cursor-pointer transition-all duration-150 ease-in-out flex justify-between items-center"
    on:click={openRecent}
    role="button"
    tabindex="0"
    on:keydown={(e) => e.key === 'Enter' && openRecent()}
    title={`Open project: ${project.name}\nPath: ${project.path}`}
  >
    <div class="min-w-0 mr-2 flex-grow">
      <h3 class="font-medium text-gray-800 group-hover:text-blue-600 truncate">{project.name}</h3>
      <p class="text-xs text-gray-500 truncate">{project.path}</p>
    </div>
    <button
      id="menu-button-for-{project.path}"
      class="flex-shrink-0 p-1 rounded-full text-gray-400 hover:bg-gray-300 hover:text-gray-700 focus:outline-none focus:ring-2 focus:ring-offset-1 focus:ring-gray-400 z-20"
      on:click|stopPropagation={toggleMenu}
      aria-label="More options for project {project.name}"
      aria-haspopup="true"
      aria-expanded={openMenuProjectPath === project.path}
    >
      <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="w-5 h-5">
        <path d="M10 3a1.5 1.5 0 1 1 0 3 1.5 1.5 0 0 1 0-3ZM10 8.5a1.5 1.5 0 1 1 0 3 1.5 1.5 0 0 1 0-3ZM11.5 15.5a1.5 1.5 0 1 0-3 0 1.5 1.5 0 0 0 3 0Z" />
      </svg>
    </button>
  </li>

  {#if openMenuProjectPath === project.path}
    <div
      class="absolute right-0 top-full mt-1 mr-2 w-40 origin-top-right rounded-md bg-white shadow-lg ring-1 ring-black ring-opacity-5 focus:outline-none z-30"
      role="menu"
      aria-orientation="vertical"
      aria-labelledby="menu-button-for-{project.path}"
      tabindex="-1"
      on:click|stopPropagation
      transition:fly={{ y: -5, duration: 150 }}
    >
      <div class="py-1" role="none">
        <button on:click={() => onMenuAction('Open')} class="text-gray-700 block w-full px-4 py-2 text-left text-sm hover:bg-gray-100" role="menuitem" tabindex="-1">Open</button>
        <button on:click={() => onMenuAction('Locate')} class="text-gray-700 block w-full px-4 py-2 text-left text-sm hover:bg-gray-100" role="menuitem" tabindex="-1">Show in Finder</button>
        <button on:click={() => onMenuAction('Rename')} class="text-gray-700 block w-full px-4 py-2 text-left text-sm hover:bg-gray-100" role="menuitem" tabindex="-1">Rename</button>
        <hr class="my-1 border-gray-200">
        <button on:click={() => onMenuAction('Remove')} class="text-red-600 block w-full px-4 py-2 text-left text-sm hover:bg-red-50" role="menuitem" tabindex="-1">Remove</button>
        <button on:click={() => onMenuAction('Delete')} class="text-red-600 block w-full px-4 py-2 text-left text-sm hover:bg-red-50" role="menuitem" tabindex="-1">Delete from Disk</button>
      </div>
    </div>
  {/if}
</div>
