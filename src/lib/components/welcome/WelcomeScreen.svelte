<!-- src/lib/components/welcome/WelcomeScreen.svelte -->
<script>
  import { onMount, onDestroy } from 'svelte';
  import { fly } from 'svelte/transition';
  import {
    loadProjects,
    handleCreateProject,
    handleOpenProject,
    handleOpenRecent,
    handleMenuAction,
    handleRenameConfirm,
    handleRenameCancel
  } from './actions.js';
  import ProjectList from './ProjectList.svelte';
  import RenameModal from './RenameModal.svelte';
  import Configure from './Configure.svelte'; // New configuration component

  // --- Reactive State Variables ---
  let recentProjects = [];
  let isLoading = true;
  let statusMessage = '';
  let openMenuProjectPath = null;
  let isRenameModalOpen = false;
  let projectToRename = null; // This will hold the *object* to rename

  // Track active tab: "projects", "configure" or "about"
  let activeTab = "projects";

  // --- State Setter Functions (Passed down to actions.js) ---
  const setRecentProjects = (projects) => { recentProjects = projects; };
  const setStatusMessage = (msg) => { statusMessage = msg; };
  const setIsLoading = (bool) => { isLoading = bool; };
  const setOpenMenu = (path) => { openMenuProjectPath = path; };
  // This function is now passed to handleMenuAction to update the local state
  const updateProjectToRename = (project) => { projectToRename = project; };
  // This function is now passed to handleMenuAction to update the local state
  const updateIsRenameModalOpen = (bool) => { isRenameModalOpen = bool; };


  // --- Lifecycle ---
  onMount(async () => {
    await loadProjects({ setRecentProjects, setStatusMessage, setIsLoading });
    document.addEventListener('click', handleClickOutside);
  });

  onDestroy(() => {
    document.removeEventListener('click', handleClickOutside);
  });

  // --- Helper: Close Menu on Outside Click ---
  function handleClickOutside(event) {
    // Ensure menu exists before trying to find closest
    if (!openMenuProjectPath) return;
    const menuElement = document.querySelector(`[data-menu-path="${openMenuProjectPath}"]`); // Assuming ProjectItem adds this data attribute
    if (menuElement && !menuElement.contains(event.target)) {
        const triggerElement = document.querySelector(`[data-menu-trigger="${openMenuProjectPath}"]`); // Button that opened it
         if (!triggerElement || !triggerElement.contains(event.target)) {
            console.log("Clicked outside menu and trigger for:", openMenuProjectPath);
            openMenuProjectPath = null; // Close the menu
         }
    } else if (!event.target.closest('[data-menu-path]')) {
        // Fallback if querySelector fails or click is truly outside any menu structure
         openMenuProjectPath = null;
    }
  }


  // --- Handlers for Button Clicks ---
  async function onCreateProject() {
    // Pass the state setters
    await handleCreateProject({ setStatusMessage, setRecentProjects, setIsLoading });
  }

  async function onOpenProject() {
     // Pass the state setters
    await handleOpenProject({ setStatusMessage, setRecentProjects, setIsLoading });
  }

  async function onOpenRecent(project) {
     // Pass the state setters
    await handleOpenRecent(project, { setStatusMessage, setRecentProjects, setIsLoading });
  }

  // Renamed handler for clarity
  async function onDispatchMenuAction(action, project) {
    // Call the action handler, passing our local state setters
    await handleMenuAction(action, project, {
      setStatusMessage,
      setRecentProjects,
      setOpenMenu,
      setProjectToRename: updateProjectToRename, // Pass the setter function
      setIsRenameModalOpen: updateIsRenameModalOpen, // Pass the setter function
      setIsLoading
    });
  }

  // Renamed handler for clarity
  async function onConfirmRename(event) {
    // Pass the state setters
    await handleRenameConfirm(event, { setStatusMessage, setRecentProjects, setIsLoading });
    // Close modal after confirm action is processed
    isRenameModalOpen = false;
    projectToRename = null;
  }

  // Renamed handler for clarity
  function onCancelRename() {
    // Pass the state setter
    handleRenameCancel({ setStatusMessage });
    // Close modal on cancel
    isRenameModalOpen = false;
    projectToRename = null;
  }

  // --- Tab Switching Handler ---
  function switchTab(tabName) {
    activeTab = tabName;
    // Reset status message when switching tabs? Optional.
    // statusMessage = '';
  }
</script>

<div class="flex h-screen bg-gray-100 font-sans text-sm">
  <!-- Sidebar -->
  <div class="w-1/4 bg-white border-r border-gray-200 p-6 flex flex-col flex-shrink-0">
    <div class="w-fit mb-8">
      <h1 class="text-2xl font-bold mb-1 text-gray-800">Harvey</h1>
      <p class="text-xs text-gray-500 relative left-[3px]">v0.1</p>
    </div>
    <nav class="flex flex-col space-y-1">
      <a
        href="#"
        class="px-3 py-2 rounded-md {activeTab === 'projects' ? 'bg-gray-200 text-gray-900 font-medium' : 'text-gray-600 hover:bg-gray-100 hover:text-gray-800'} text-sm"
        on:click|preventDefault={() => switchTab('projects')}
      >
        Projects
      </a>
      <a
        href="#"
        class="px-3 py-2 rounded-md {activeTab === 'configure' ? 'bg-gray-200 text-gray-900 font-medium' : 'text-gray-600 hover:bg-gray-100 hover:text-gray-800'} text-sm"
        on:click|preventDefault={() => switchTab('configure')}
      >
        Configure
      </a>
      <a
        href="#"
        class="px-3 py-2 rounded-md {activeTab === 'about' ? 'bg-gray-200 text-gray-900 font-medium' : 'text-gray-600 hover:bg-gray-100 hover:text-gray-800'} text-sm"
        on:click|preventDefault={() => switchTab('about')}
      >
        About
      </a>
    </nav>
    <div class="mt-auto text-xs text-gray-500 truncate" title={statusMessage}>
      Status: {statusMessage || 'Ready'}
    </div>
  </div>

  <!-- Main Content -->
  <div class="w-3/4 p-8 flex flex-col overflow-hidden">
    {#if activeTab === 'projects'}
      <div class="flex justify-end mb-6 space-x-3 flex-shrink-0">
        <button
          on:click={onCreateProject}
          class="px-4 py-2 bg-blue-600 text-white rounded-md shadow-sm hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-opacity-75 transition duration-150 ease-in-out text-sm font-medium"
        >
          Create Project
        </button>
        <button
          on:click={onOpenProject}
          class="px-4 py-2 bg-gray-200 text-gray-800 rounded-md shadow-sm hover:bg-gray-300 focus:outline-none focus:ring-2 focus:ring-gray-400 focus:ring-opacity-75 transition duration-150 ease-in-out text-sm font-medium"
        >
          Open Project...
        </button>
      </div>

      <div class="flex-grow overflow-y-auto bg-white p-6 rounded-lg border border-gray-200 shadow-inner">
        <h2 class="text-lg font-semibold mb-4 text-gray-700 border-b border-gray-200 pb-2">Recent Projects</h2>
        {#if isLoading}
            <p class="text-center text-gray-500 py-4">Loading projects...</p>
        {:else if recentProjects.length === 0}
            <p class="text-center text-gray-500 py-4">No recent projects found.</p>
            <p class="text-center text-gray-400 text-xs py-1">Create a new project or open an existing one.</p>
        {:else}
            <ProjectList
              {recentProjects}
              {openMenuProjectPath}
              on:openRecent={({ detail: project }) => onOpenRecent(project)}
              on:menuAction={({ detail }) => onDispatchMenuAction(detail.action, detail.project)}
              on:toggleMenu={({ detail: path }) => (openMenuProjectPath = (openMenuProjectPath === path ? null : path))}
            />
        {/if}
      </div>
    {:else if activeTab === 'configure'}
      <Configure />
    {:else if activeTab === 'about'}
      <!-- For demonstration, we'll just show a simple About text -->
      <div class="flex-grow overflow-y-auto bg-white p-6 rounded-lg border border-gray-200 shadow-inner">
        <h2 class="text-lg font-semibold mb-4 text-gray-700 border-b border-gray-200 pb-2">About Harvey</h2>
        <p class="text-gray-600">Harvey is a desktop application for local audio/video transcription with speaker diarization. Version 0.1.</p>
        <!-- Add more about info here if needed -->
      </div>
    {/if}
  </div>
</div>

<!-- Pass the reactive variables for modal state and data -->
<RenameModal
  bind:showModal={isRenameModalOpen}
  projectToRename={projectToRename}
  on:confirm={onConfirmRename}
  on:cancel={onCancelRename}
/>

<style>
  /* Custom scrollbar for Webkit browsers */
  .overflow-y-auto::-webkit-scrollbar {
    width: 6px;
  }
  .overflow-y-auto::-webkit-scrollbar-track {
    background: transparent;
  }
  .overflow-y-auto::-webkit-scrollbar-thumb {
    background-color: rgba(156, 163, 175, 0.5);
    border-radius: 10px;
    border: 3px solid transparent;
    background-clip: content-box;
  }
  .overflow-y-auto::-webkit-scrollbar-thumb:hover {
    background-color: rgba(107, 114, 128, 0.6);
  }

  /* Custom scrollbar for Firefox */
  .overflow-y-auto {
    scrollbar-width: thin;
    scrollbar-color: rgba(156, 163, 175, 0.5) transparent;
  }
</style>