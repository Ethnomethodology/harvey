<!-- src/lib/components/welcome/ProjectList.svelte -->
<script>
  import ProjectItem from './ProjectItem.svelte';
  export let recentProjects = [];
  export let isLoading = false;
  export let openMenuProjectPath = null;

  // Forward event details to parent component.
  const handleOpenRecent = (project) => {
    dispatch('openRecent', project);
  };

  const handleMenuAction = (action, project) => {
    dispatch('menuAction', { action, project });
  };

  const toggleMenu = (path) => {
    dispatch('toggleMenu', path);
  };

  import { createEventDispatcher } from 'svelte';
  const dispatch = createEventDispatcher();
</script>

{#if isLoading}
  <p class="text-gray-500 text-center py-4">Loading projects...</p>
{:else if recentProjects.length > 0}
  <ul class="space-y-2">
    {#each recentProjects as project (project.path)}
      <ProjectItem
        {project}
        {openMenuProjectPath}
        on:openRecent={(e) => dispatch('openRecent', e.detail)}
        on:menuAction={(e) => dispatch('menuAction', e.detail)}
        on:toggleMenu={(e) => dispatch('toggleMenu', e.detail)}
      />
    {/each}
  </ul>
{:else}
  <div class="flex justify-center items-center h-full py-10">
    <p class="text-gray-500 text-center">
      No recent projects found.<br />
      Click 'Create Project' or 'Open Project...' to get started.
    </p>
  </div>
{/if}
