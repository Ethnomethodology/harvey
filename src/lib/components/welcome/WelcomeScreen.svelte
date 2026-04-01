<!-- src/lib/components/welcome/WelcomeScreen.svelte -->
<script>
  import { onMount, onDestroy } from 'svelte';
  import { fly } from 'svelte/transition';
  import { invoke } from '@tauri-apps/api/core'; // Added
  import { listen } from '@tauri-apps/api/event'; // Added
  import { getVersion } from '@tauri-apps/api/app';
  import {
    getCurrentWindow,
    LogicalSize,
    currentMonitor,
  } from '@tauri-apps/api/window';
  import {
    WELCOME_WIDTH,
    DEFAULT_MIN_HEIGHT,
  } from '$lib/constants/windowSize.js';
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
  import HelpModal from '$lib/components/modals/HelpModal.svelte';
  import ConfigurationView from '$lib/components/shared/ConfigurationView.svelte';
  import AboutContent from './AboutContent.svelte';
  import SetupWizardModal from './SetupWizardModal.svelte';

  // --- Reactive State Variables ---
  let recentProjects = [];
  let isLoading = true;
  let openMenuProjectPath = null;
  let isRenameModalOpen = false;
  let showHelpModal = false;
  let showWizardModal = false;
  let projectToRename = null; // This will hold the *object* to rename

  // Track active tab: "projects", "configure" or "about"
  let activeTab = "projects";
  let appVersion = "";

  // --- State Setter Functions (Passed down to actions.js) ---
  const setRecentProjects = (projects) => { recentProjects = projects; };
  const setIsLoading = (bool) => { isLoading = bool; };
  const setOpenMenu = (path) => { openMenuProjectPath = path; };
  // This function is now passed to handleMenuAction to update the local state
  const updateProjectToRename = (project) => { projectToRename = project; };
  // This function is now passed to handleMenuAction to update the local state
  const updateIsRenameModalOpen = (bool) => { isRenameModalOpen = bool; };


  let unlistenNewProject;
  let unlistenOpenProject;
  let unlistenHelpCenter;

  onMount(async () => {
    // Resize window for welcome screen
    try {
      const appWindow = getCurrentWindow();
      await appWindow.setMinSize(
        new LogicalSize(WELCOME_WIDTH, DEFAULT_MIN_HEIGHT),
      );
      await appWindow.setSize(new LogicalSize(WELCOME_WIDTH, WELCOME_HEIGHT));
      await appWindow.center();
    } catch (err) {
      console.warn('Failed to resize welcome window:', err);
    }

    // Load projects first so the UI isn't blocked by other initialization
    console.log('[WelcomeScreen] onMount: Loading projects...');
    await loadProjects({ setRecentProjects, setIsLoading });
    console.log('[WelcomeScreen] onMount: Projects loaded.');

    await invoke('set_menu_context', { context: 'welcome' }).catch(err => console.warn('Failed to set menu context:', err));
    
    console.log('[WelcomeScreen] onMount: Setting up event listeners...');
    unlistenNewProject = await listen('menu:file:new-project', () => {
        onCreateProject();
    });
    unlistenOpenProject = await listen('menu:file:open-project', () => {
        onOpenProject();
    });
    unlistenHelpCenter = await listen('menu:help:center', () => {
        showHelpModal = true;
    });
    
    try {
        console.log('[WelcomeScreen] onMount: Getting app version...');
        appVersion = await getVersion();
        console.log('[WelcomeScreen] onMount: App version obtained:', appVersion);
    } catch (err) {
        console.warn('Failed to get app version:', err);
    }

    document.addEventListener('click', handleClickOutside);
  });

  onDestroy(() => {
    if (unlistenNewProject) unlistenNewProject();
    if (unlistenOpenProject) unlistenOpenProject();
    if (unlistenHelpCenter) unlistenHelpCenter();
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

  $: hasCriticalConfigIssues = !$configStatus.python_libraries_installed;
  $: hasNonCriticalConfigIssues = !hasCriticalConfigIssues && (!$configStatus.hf_token_present || !$configStatus.transcription_models_downloaded || !$configStatus.diarization_model_downloaded || !$configStatus.translation_models_downloaded);
</script>

<div class="flex h-screen bg-gray-100 dark:bg-gray-950 font-sans text-sm">
  <!-- Sidebar -->
  <div class="w-1/4 bg-white dark:bg-gray-900 border-r border-gray-200 dark:border-gray-700 p-6 flex flex-col flex-shrink-0">
    <div class="w-fit mb-8">
      <h1 class="text-2xl font-bold mb-1 text-gray-800 dark:text-gray-100">Harvey</h1>
      <p class="text-xs text-gray-500 dark:text-gray-400 relative left-[3px]">v{appVersion || '0.1'}</p>
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
        class="px-3 py-2 rounded-md transition-colors text-sm flex items-center justify-between
          {activeTab === 'configure' ? 'bg-gray-200 dark:bg-gray-700 text-gray-900 dark:text-gray-100 font-medium' : 'text-gray-600 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700/50 hover:text-gray-800 dark:hover:text-gray-200'}"
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
    <div class="mt-auto">
      <button
        class="w-full flex items-center px-3 py-2 rounded-md text-gray-600 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700/50 hover:text-gray-800 dark:hover:text-gray-200 text-sm font-medium transition-colors"
        on:click={() => showHelpModal = true}
      >
        <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4 mr-2" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8.228 9c.549-1.165 2.03-2 3.772-2 2.21 0 4 1.343 4 3 0 1.4-1.278 2.575-3.006 2.907-.542.104-.994.54-.994 1.093m0 3h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
        </svg>
        Help Center
      </button>
    </div>
  </div>

  <!-- Main Content -->
  <div class="w-3/4 p-8 flex flex-col overflow-hidden">
    {#if activeTab === 'projects'}
      {#if hasCriticalConfigIssues || hasNonCriticalConfigIssues}
        <div class="mb-6 bg-yellow-50 dark:bg-yellow-900/20 border border-yellow-200 dark:border-yellow-800 rounded-xl p-5 flex items-start space-x-4 flex-shrink-0 shadow-sm">
          <div class="p-2 bg-yellow-100 dark:bg-yellow-900/40 rounded-lg">
            <svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6 text-yellow-600 dark:text-yellow-500" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
            </svg>
          </div>
          <div class="flex-grow">
            <h3 class="text-base font-bold text-yellow-900 dark:text-yellow-100 uppercase tracking-tight">Configuration Required</h3>
            <p class="text-sm text-yellow-800 dark:text-yellow-300 mt-1 leading-relaxed">
              Your installation is almost complete! Some required libraries or AI models are missing. Please complete the setup to enable full transcription and translation features.
            </p>
            <div class="mt-4 flex items-center space-x-4">
              {#if hasCriticalConfigIssues}
                <button
                  on:click={() => showWizardModal = true}
                  class="px-5 py-2 bg-yellow-600 hover:bg-yellow-700 text-white rounded-lg font-bold shadow-md shadow-yellow-600/20 transition-all flex items-center space-x-2"
                >
                  <span>Launch Setup Wizard</span>
                  <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 7l5 5m0 0l-5 5m5-5H6" />
                  </svg>
                </button>
                <button
                  on:click={() => switchTab('configure')}
                  class="text-xs font-bold text-yellow-700 dark:text-yellow-400 hover:text-yellow-900 dark:hover:text-yellow-200 underline decoration-2 underline-offset-4 transition-colors"
                >
                  Manually Configure
                </button>
              {:else}
                <button
                  on:click={() => switchTab('configure')}
                  class="text-sm font-bold text-yellow-700 dark:text-yellow-400 hover:text-yellow-900 dark:hover:text-yellow-200 underline decoration-2 underline-offset-4 transition-colors"
                >
                  Switch to Configure tab
                </button>
              {/if}
            </div>
          </div>
        </div>
      {/if}

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

      <div class="flex-grow overflow-y-auto bg-white dark:bg-gray-900 p-6 rounded-lg border border-gray-200 dark:border-gray-700 shadow-inner">
        <h2 class="text-lg font-semibold mb-4 text-gray-700 dark:text-gray-200 border-b border-gray-200 dark:border-gray-700 pb-2">Recent Projects</h2>
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
      <div class="flex-grow flex flex-col overflow-hidden h-full">
          <AboutContent />
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

<HelpModal bind:showModal={showHelpModal} on:close={() => showHelpModal = false} />

<SetupWizardModal bind:showModal={showWizardModal} />

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