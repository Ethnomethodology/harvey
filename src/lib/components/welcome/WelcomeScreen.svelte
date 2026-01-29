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
  import { configStatus } from '$lib/stores/configStatusStore.js';
  import ProjectList from './ProjectList.svelte';
  import RenameModal from './RenameModal.svelte';
  import ConfigurationView from '$lib/components/shared/ConfigurationView.svelte';

  // --- Reactive State Variables ---
  let recentProjects = [];
  let isLoading = true;
  let openMenuProjectPath = null;
  let isRenameModalOpen = false;
  let projectToRename = null; // This will hold the *object* to rename

  // Track active tab: "projects", "configure" or "about"
  let activeTab = "projects";

  // --- State Setter Functions (Passed down to actions.js) ---
  const setRecentProjects = (projects) => { recentProjects = projects; };
  const setIsLoading = (bool) => { isLoading = bool; };
  const setOpenMenu = (path) => { openMenuProjectPath = path; };
  // This function is now passed to handleMenuAction to update the local state
  const updateProjectToRename = (project) => { projectToRename = project; };
  // This function is now passed to handleMenuAction to update the local state
  const updateIsRenameModalOpen = (bool) => { isRenameModalOpen = bool; };


  // --- Lifecycle ---
  onMount(async () => {
    await loadProjects({ setRecentProjects, setIsLoading });
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
    await handleCreateProject({ setRecentProjects, setIsLoading });
  }

  async function onOpenProject() {
     // Pass the state setters
    await handleOpenProject({ setRecentProjects, setIsLoading });
  }

  async function onOpenRecent(project) {
     // Pass the state setters
    await handleOpenRecent(project, { setRecentProjects, setIsLoading });
  }

  // Renamed handler for clarity
  async function onDispatchMenuAction(action, project) {
    // Call the action handler, passing our local state setters
    await handleMenuAction(action, project, {
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
    await handleRenameConfirm(event, { setRecentProjects, setIsLoading });
    // Close modal after confirm action is processed
    isRenameModalOpen = false;
    projectToRename = null;
  }

  // Renamed handler for clarity
  function onCancelRename() {
    // Pass the state setter
    handleRenameCancel();
    // Close modal on cancel
    isRenameModalOpen = false;
    projectToRename = null;
  }

  // --- Tab Switching Handler ---
  function switchTab(tabName) {
    activeTab = tabName;
  }

  $: hasCriticalConfigIssues = !$configStatus.python_libraries_installed || !$configStatus.transcription_models_downloaded;
  $: hasNonCriticalConfigIssues = !hasCriticalConfigIssues && (!$configStatus.hf_token_present || !$configStatus.diarization_model_downloaded || !$configStatus.translation_models_downloaded);
</script>

<div class="flex h-screen bg-gray-100 dark:bg-surface-1 font-sans text-sm">
  <!-- Sidebar -->
  <div class="w-1/4 bg-white dark:bg-surface-2 border-r border-gray-200 dark:border-border p-6 flex flex-col flex-shrink-0">
    <div class="w-fit mb-8">
      <h1 class="text-2xl font-bold mb-1 text-gray-800 dark:text-gray-100">Harvey</h1>
      <p class="text-xs text-gray-500 dark:text-gray-400 relative left-[3px]">v0.1</p>
    </div>
    <nav class="flex flex-col space-y-1">
      <a
        href="#"
        class="px-3 py-2 rounded-md {activeTab === 'projects' ? 'bg-gray-200 dark:bg-gray-700 text-gray-900 dark:text-gray-100 font-medium' : 'text-gray-600 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700/50 hover:text-gray-800 dark:hover:text-gray-200'} text-sm"
        on:click|preventDefault={() => switchTab('projects')}
      >
        Projects
      </a>
      <a
        href="#"
        class="px-3 py-2 rounded-md {activeTab === 'configure' ? 'bg-gray-200 dark:bg-gray-700 text-gray-900 dark:text-gray-100 font-medium' : 'text-gray-600 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700/50 hover:text-gray-800 dark:hover:text-gray-200'} text-sm flex items-center justify-between"
        on:click|preventDefault={() => switchTab('configure')}
      >
        <span>Configure</span>
        {#if hasCriticalConfigIssues}
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="h-4 w-4 text-red-500" viewBox="0 0 16 16">
            <path d="M6.95.435c.58-.58 1.52-.58 2.1 0l6.515 6.516c.58.58.58 1.519 0 2.098L9.05 15.565c-.58.58-1.519.58-2.098 0L.435 9.05a1.48 1.48 0 0 1 0-2.098zm1.4.7a.495.495 0 0 0-.7 0L1.134 7.65a.495.495 0 0 0 0 .7l6.516 6.516a.495.495 0 0 0 .7 0l6.516-6.516a.495.495 0 0 0 0-.7L8.35 1.134z"/>
            <path d="M7.002 11a1 1 0 1 1 2 0 1 1 0 0 1-2 0M7.1 4.995a.905.905 0 1 1 1.8 0l-.35 3.507a.552.552 0 0 1-1.1 0z"/>
          </svg>
        {:else if hasNonCriticalConfigIssues}
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="h-4 w-4 text-yellow-500" viewBox="0 0 16 16">
            <path d="M6.95.435c.58-.58 1.52-.58 2.1 0l6.515 6.516c.58.58.58 1.519 0 2.098L9.05 15.565c-.58.58-1.519.58-2.098 0L.435 9.05a1.48 1.48 0 0 1 0-2.098zm1.4.7a.495.495 0 0 0-.7 0L1.134 7.65a.495.495 0 0 0 0 .7l6.516 6.516a.495.495 0 0 0 .7 0l6.516-6.516a.495.495 0 0 0 0-.7L8.35 1.134z"/>
            <path d="M7.002 11a1 1 0 1 1 2 0 1 1 0 0 1-2 0M7.1 4.995a.905.905 0 1 1 1.8 0l-.35 3.507a.552.552 0 0 1-1.1 0z"/>
          </svg>
        {/if}
      </a>
      <a
        href="#"
        class="px-3 py-2 rounded-md {activeTab === 'about' ? 'bg-gray-200 dark:bg-gray-700 text-gray-900 dark:text-gray-100 font-medium' : 'text-gray-600 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700/50 hover:text-gray-800 dark:hover:text-gray-200'} text-sm"
        on:click|preventDefault={() => switchTab('about')}
      >
        About
      </a>
    </nav>
  </div>

  <!-- Main Content -->
  <div class="w-3/4 p-8 flex flex-col overflow-hidden">
    {#if activeTab === 'projects'}
      <div class="flex justify-end mb-6 space-x-3 flex-shrink-0">
        <button
          on:click={onCreateProject}
          class="px-4 py-2 bg-blue-600 dark:bg-blue-700 text-white rounded-md shadow-sm hover:bg-blue-700 dark:hover:bg-blue-600 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-opacity-75 transition duration-150 ease-in-out text-sm font-medium"
        >
          Create Project
        </button>
        <button
          on:click={onOpenProject}
          class="px-4 py-2 bg-gray-200 dark:bg-gray-600 text-gray-800 dark:text-gray-100 rounded-md shadow-sm hover:bg-gray-300 dark:hover:bg-gray-500 focus:outline-none focus:ring-2 focus:ring-gray-400 focus:ring-opacity-75 transition duration-150 ease-in-out text-sm font-medium"
        >
          Open Project...
        </button>
      </div>

      <div class="flex-grow overflow-y-auto bg-white dark:bg-surface-2 p-6 rounded-lg border border-gray-200 dark:border-border shadow-inner">
        <h2 class="text-lg font-semibold mb-4 text-gray-700 dark:text-gray-200 border-b border-gray-200 dark:border-border pb-2">Recent Projects</h2>
        {#if isLoading}
            <p class="text-center text-gray-500 dark:text-gray-400 py-4">Loading projects...</p>
        {:else if recentProjects.length === 0}
            <p class="text-center text-gray-500 dark:text-gray-400 py-4">No recent projects found.</p>
            <p class="text-center text-gray-400 dark:text-gray-500 text-xs py-1">Create a new project or open an existing one.</p>
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
      <ConfigurationView />
    {:else if activeTab === 'about'}
      <!-- For demonstration, we'll just show a simple About text -->
      <div class="flex-grow overflow-y-auto bg-white dark:bg-surface-2 p-6 rounded-lg border border-gray-200 dark:border-border shadow-inner">
        <h2 class="text-lg font-semibold mb-4 text-gray-700 dark:text-gray-200 border-b border-gray-200 dark:border-border pb-2">About Harvey</h2>

        <p class="text-gray-600 dark:text-gray-300 mb-2">Harvey is a desktop application for qualitative researchers and anyone working with multimedia content. It streamlines the research workflow by integrating a powerful suite of tools into a single, cohesive environment:</p>
        <ul class="list-disc list-inside text-gray-600 dark:text-gray-300 mb-2 pl-4">
            <li><strong>Transcribe</strong> audio and video with local AI models.</li>
            <li><strong>Translate</strong> transcripts into English.</li>
            <li><strong>Edit</strong> and <strong>annotate</strong> transcripts, documents, PDFs, and images.</li>
            <li><strong>Manage</strong> all your project files in one place.</li>
        </ul>
        <p class="text-gray-600 dark:text-gray-300 mb-2">Harvey is named in honor of Sociologist <a href="https://emcawiki.net/bibtex/browser.php?author=Harvey+Sacks&bib=emca.bib" target="_blank" rel="noopener noreferrer" class="text-blue-500 dark:text-blue-400 hover:underline"><strong>Harvey Sacks</strong></a> (1935-1975), whose foundational work revolutionized social research methods.</p>
        <h4 class="text-md font-semibold mt-4 mb-2 text-gray-700 dark:text-gray-200">Privacy First</h4>
        <p class="text-gray-600 dark:text-gray-300 mb-2">Privacy is the core of Harvey's design. All core AI functionalities, like transcription and diarization (speaker identification), run <strong>100% locally</strong> on your computer. You download the AI models once and can use them forever offline. Your data never leaves your device.</p>
        <blockquote class="border-l-4 border-gray-300 dark:border-gray-500 pl-4 italic text-gray-600 dark:text-gray-400 mb-4">It's an application built <em>by researchers, for researchers</em>.</blockquote>
      </div>
      <div class="flex-shrink-0 mt-auto text-center py-4">
        <p class="text-gray-600 dark:text-gray-400">
            <a href="https://github.com/Ethnomethodology/harvey" target="_blank" rel="noopener noreferrer" class="text-blue-500 dark:text-blue-400 hover:underline">Harvey</a>
            <span class="mx-1">•</span>
            <a href="https://github.com/Ethnomethodology/harvey/blob/main/LICENSE.md" target="_blank" rel="noopener noreferrer" class="text-blue-500 dark:text-blue-400 hover:underline">MIT License</a>
        </p>
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