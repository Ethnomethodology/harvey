<!-- src/lib/components/modals/HelpModal.svelte -->
<script>
  import { createEventDispatcher, onMount } from 'svelte';
  import AboutBody from '$lib/components/shared/AboutBody.svelte';

  export let showModal = false;

  const dispatch = createEventDispatcher();

  // Determine platform-specific modifier key name
  const isMac = typeof window !== 'undefined' && navigator.platform.toUpperCase().indexOf('MAC') >= 0;
  const modKeyName = isMac ? 'Cmd' : 'Ctrl';

  // Define the flat sequence of all pages using exact UI strings
  const allPages = [
    { id: 'overview', label: 'Help Center', sidebarId: 'overview' },
    { id: 'about-harvey', label: 'About Harvey', sidebarId: 'overview' },
    { id: 'getting-started', label: 'Getting Started', sidebarId: 'overview' },
    { id: 'projects', label: 'Projects', sidebarId: 'overview' },
    { id: 'supported-platforms', label: 'Supported Platforms', sidebarId: 'overview' },
    
    { id: 'configure', label: 'Configure', sidebarId: 'configure' },
    { id: 'config-app', label: 'Application', sidebarId: 'configure' },
    { id: 'config-transcription', label: 'Transcription Engine', sidebarId: 'configure' },
    { id: 'config-diarization', label: 'Diarization Engine', sidebarId: 'configure' },
    { id: 'config-translation', label: 'Translation Engine', sidebarId: 'configure' },
    { id: 'config-advanced', label: 'Advanced Settings', sidebarId: 'configure' },

    { id: 'manage-data', label: 'Data', sidebarId: 'manage-data' },
    { id: 'data-tab', label: 'Data Tab', sidebarId: 'manage-data' },
    { id: 'audio', label: 'Audios', sidebarId: 'manage-data' },
    { id: 'video', label: 'Videos', sidebarId: 'manage-data' },
    { id: 'documents', label: 'Documents', sidebarId: 'manage-data' },
    { id: 'images', label: 'Images', sidebarId: 'manage-data' },
    { id: 'tables', label: 'Tables', sidebarId: 'manage-data' },
    { id: 'transcripts', label: 'Transcripts', sidebarId: 'manage-data' },
    { id: 'groups', label: 'Groups', sidebarId: 'manage-data' },

    { id: 'transcribe', label: 'Transcription', sidebarId: 'transcribe' },
    { id: 'transcription-tab', label: 'Transcription Tab', sidebarId: 'transcribe' },
    { id: 'auto-transcription', label: 'Automatic Transcription', sidebarId: 'transcribe' },
    { id: 'manual-transcription', label: 'Manual Transcription', sidebarId: 'transcribe' },
    { id: 'diarization', label: 'Diarization', sidebarId: 'transcribe' },
    { id: 'translation', label: 'Translation', sidebarId: 'transcribe' },
    { id: 'media-player', label: 'Media Player', sidebarId: 'transcribe' },
    { id: 'editing', label: 'Editing', sidebarId: 'transcribe' },
    { id: 'waveform', label: 'Waveform', sidebarId: 'transcribe' },
    { id: 'layout', label: 'Layout', sidebarId: 'transcribe' },
    { id: 'shortcuts', label: 'Shortcuts', sidebarId: 'transcribe' },
    { id: 'export', label: 'Export', sidebarId: 'transcribe' },

    { id: 'tags', label: 'Tags', sidebarId: 'tags' },
    { id: 'tags-tab', label: 'Tags Tab', sidebarId: 'tags' },
    { id: 'highlights', label: 'Highlights', sidebarId: 'tags' },
    { id: 'tag-groups', label: 'Tag Groups', sidebarId: 'tags' },
    
    { id: 'annotate-page', label: 'Annotate', sidebarId: 'annotate-page' },
    { id: 'media-annotations', label: 'Media Annotations', sidebarId: 'annotate-page' },
    { id: 'document-annotations', label: 'Document Annotations', sidebarId: 'annotate-page' },
    { id: 'pdf-annotations', label: 'PDF Annotations', sidebarId: 'annotate-page' },
    { id: 'image-annotations', label: 'Image Annotations', sidebarId: 'annotate-page' },
    { id: 'table-annotations', label: 'Table Annotations', sidebarId: 'annotate-page' },
    { id: 'transcript-annotations', label: 'Transcript Annotations', sidebarId: 'annotate-page' },
    { id: 'highlights-panel', label: 'Highlights Panel', sidebarId: 'annotate-page' },

    { id: 'translate-page', label: 'Translate', sidebarId: 'translate-page' },
    { id: 'translate-transcript', label: 'Translating Transcript', sidebarId: 'translate-page' },
    { id: 'translate-document', label: 'Translating Document', sidebarId: 'translate-page' },
    { id: 'translate-models', label: 'Translation Models', sidebarId: 'translate-page' },

    { id: 'report-issue', label: 'Report Issue', sidebarId: 'report-issue' }
  ];

  // Sidebar tabs (only top-level sections)
  const sidebarTabs = [
    { id: 'overview', label: 'Overview' },
    { id: 'configure', label: 'Configure' },
    { id: 'manage-data', label: 'Data' },
    { id: 'transcribe', label: 'Transcription' },
    { id: 'tags', label: 'Tags' },
    { id: 'annotate-page', label: 'Annotate' },
    { id: 'translate-page', label: 'Translate' },
    { id: 'report-issue', label: 'Report Issue' }
  ];

  let currentPageId = 'overview';
  let searchQuery = '';
  let showSearchResults = false;

  $: filteredResults = searchQuery.trim() === '' 
    ? [] 
    : allPages.filter(p => p.label.toLowerCase().includes(searchQuery.toLowerCase()));

  $: currentIndex = allPages.findIndex(p => p.id === currentPageId);
  $: currentPage = allPages[currentIndex];
  $: prevPage = currentIndex > 0 ? allPages[currentIndex - 1] : null;
  $: nextPage = currentIndex < allPages.length - 1 ? allPages[currentIndex + 1] : null;

  function close() {
    dispatch('close');
  }

  function handleKeydown(event) {
    if (event.key === 'Escape') {
      if (showSearchResults) {
        showSearchResults = false;
      }
    }
  }

  function navigateTo(pageId) {
    currentPageId = pageId;
    searchQuery = '';
    showSearchResults = false;
  }

  function goToPrev() {
    if (prevPage) navigateTo(prevPage.id);
  }

  function goToNext() {
    if (nextPage) navigateTo(nextPage.id);
  }

  const ICON_PREV = `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-chevron-compact-left" viewBox="0 0 16 16"><path fill-rule="evenodd" d="M9.224 1.553a.5.5 0 0 1 .223.67L6.56 8l2.888 5.776a.5.5 0 1 1-.894.448l-3-6a.5.5 0 0 1 0-.448l3-6a.5.5 0 0 1 .67-.223"/></svg>`;
  const ICON_NEXT = `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-chevron-compact-right" viewBox="0 0 16 16"><path fill-rule="evenodd" d="M6.776 1.553a.5.5 0 0 1 .671.223l3 6a.5.5 0 0 1 0 .448l-3 6a.5.5 0 1 1-.894-.448L9.44 8 6.553 2.224a.5.5 0 0 1 .223-.671"/></svg>`;

  onMount(() => {
    const listener = (e) => {
        if (!e.target.closest('.help-search-container')) {
            showSearchResults = false;
        }
    };
    document.addEventListener('click', listener);
    return () => document.removeEventListener('click', listener);
  });
</script>

{#if showModal}
  <div
    class="fixed inset-0 z-[150] flex items-center justify-center bg-gray-900 bg-opacity-60 backdrop-blur-sm"
    on:keydown={handleKeydown}
    role="dialog"
    aria-modal="true"
    aria-labelledby="help-modal-title"
    tabindex="-1"
  >
    <div class="bg-white dark:bg-surface-2 rounded-lg shadow-xl w-[80vw] h-[80vh] flex flex-col relative overflow-hidden" role="document">
      
      <!-- Main Modal Header -->
      <div class="flex-shrink-0 px-6 py-4 border-b border-gray-200 dark:border-border flex justify-between items-center bg-gray-50 dark:bg-dark-bg-secondary">
        <div class="flex items-center space-x-3">
            <img src="/logo.png" alt="Harvey Logo" class="w-8 h-8 rounded-lg" />
            <h2 id="help-modal-title" class="text-xl font-bold text-gray-800 dark:text-white">Help Center</h2>
        </div>
        
        <div class="flex items-center space-x-4 flex-grow justify-end">
            <!-- Search Bar -->
            <div class="relative w-64 help-search-container">
                <div class="relative">
                    <input 
                        type="text" 
                        bind:value={searchQuery}
                        on:focus={() => showSearchResults = true}
                        placeholder="Search help..." 
                        autocomplete="off"
                        autocorrect="off"
                        autocapitalize="off"
                        spellcheck="false"
                        class="w-full pl-9 pr-4 py-1.5 text-sm bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-600 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 dark:text-white transition-all"
                    />
                    <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4 absolute left-3 top-1/2 -translate-y-1/2 text-gray-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
                    </svg>
                </div>

                <!-- Search Results Dropdown -->
                {#if showSearchResults && filteredResults.length > 0}
                    <div class="absolute top-full mt-1 left-0 right-0 bg-white dark:bg-surface-3 border border-gray-200 dark:border-border rounded-lg shadow-xl z-[160] py-1 max-h-64 overflow-y-auto">
                        {#each filteredResults as result}
                            <button 
                                class="w-full text-left px-4 py-2 text-sm hover:bg-blue-50 dark:hover:bg-blue-900/30 dark:text-gray-200 transition-colors flex items-center justify-between group"
                                on:click={() => navigateTo(result.id)}
                            >
                                <span>{result.label}</span>
                                <span class="text-[10px] text-gray-400 group-hover:text-blue-500 uppercase tracking-wider">{result.sidebarId}</span>
                            </button>
                        {/each}
                    </div>
                {:else if showSearchResults && searchQuery.trim() !== ''}
                    <div class="absolute top-full mt-1 left-0 right-0 bg-white dark:bg-surface-3 border border-gray-200 dark:border-border rounded-lg shadow-xl z-[160] p-4 text-center">
                        <p class="text-sm text-gray-500">No results found for "{searchQuery}"</p>
                    </div>
                {/if}
            </div>

            <button 
                on:click={close} 
                aria-label="Close" 
                class="text-gray-400 hover:text-gray-600 dark:hover:text-gray-200 focus:outline-none focus:ring-2 focus:ring-gray-400 rounded-full p-1 transition-colors"
            >
                <svg xmlns="http://www.w3.org/2000/svg" class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                </svg>
            </button>
        </div>
      </div>

      <!-- Main Body -->
      <div class="flex flex-grow overflow-hidden">
        <!-- Sidebar -->
        <div class="w-64 bg-gray-50 dark:bg-dark-bg-secondary border-r border-gray-200 dark:border-border p-4 flex flex-col flex-shrink-0 overflow-y-auto font-sans">
          <nav class="flex flex-col space-y-1">
            {#each sidebarTabs as tab}
              <button
                class="px-4 py-2 rounded-md text-left transition-all duration-200 {currentPage.sidebarId === tab.id 
                  ? 'bg-blue-600 text-white font-medium shadow-sm' 
                  : 'text-gray-600 dark:text-gray-300 hover:bg-gray-200 dark:hover:bg-gray-700/50 hover:text-gray-900 dark:hover:text-gray-100'} text-sm"
                on:click={() => navigateTo(tab.id)}
              >
                {tab.label}
              </button>
            {/each}
          </nav>
        </div>

        <!-- Content Area Container -->
        <div class="flex-grow flex flex-col overflow-hidden bg-white dark:bg-surface-2 font-sans">
          
          <!-- Inner Page Header (Navigation) -->
          <div class="flex-shrink-0 px-8 py-3 border-b border-gray-100 dark:border-gray-800 flex justify-between items-center bg-gray-50 dark:bg-dark-bg-secondary h-12">
            <button 
                class="flex items-center space-x-1 text-sm font-medium text-gray-500 hover:text-blue-600 dark:text-gray-400 dark:hover:text-blue-400 transition-colors disabled:opacity-0"
                on:click={goToPrev}
                disabled={!prevPage}
            >
                <span class="flex-shrink-0">{@html ICON_PREV}</span>
                <span>Previous: {prevPage?.label || ''}</span>
            </button>

            <button 
                class="flex items-center space-x-1 text-sm font-medium text-gray-500 hover:text-blue-600 dark:text-gray-400 dark:hover:text-blue-400 transition-colors disabled:opacity-0"
                on:click={goToNext}
                disabled={!nextPage}
            >
                <span>Next: {nextPage?.label || ''}</span>
                <span class="flex-shrink-0">{@html ICON_NEXT}</span>
            </button>
          </div>

          <!-- Scrollable Page Content -->
          <div class="flex-grow overflow-y-auto p-8 text-gray-700 dark:text-gray-300">
            
            {#if currentPageId === 'overview'}
              <div class="space-y-8">
                <div>
                    <h3 class="text-3xl font-extrabold mb-2 text-gray-900 dark:text-white">Help Center</h3>
                    <p class="text-lg text-gray-500 dark:text-gray-400">Everything you need to work with Harvey.</p>
                </div>

                <div class="grid grid-cols-1 md:grid-cols-2 gap-4 pb-8">
                    <button 
                        on:click={() => navigateTo('about-harvey')}
                        class="flex flex-col p-6 text-left border border-gray-200 dark:border-gray-700 rounded-xl hover:border-blue-500 dark:hover:border-blue-500 hover:shadow-md transition-all group bg-gray-50/50 dark:bg-surface-3/30"
                    >
                        <span class="text-xl font-bold mb-2 group-hover:text-blue-600 dark:group-hover:text-blue-400 text-gray-800 dark:text-gray-100">About Harvey</span>
                        <p class="text-sm text-gray-500 dark:text-gray-400">Learn about the vision and core features of the platform.</p>
                    </button>

                    <button 
                        on:click={() => navigateTo('getting-started')}
                        class="flex flex-col p-6 text-left border border-gray-200 dark:border-gray-700 rounded-xl hover:border-blue-500 dark:hover:border-blue-500 hover:shadow-md transition-all group bg-gray-50/50 dark:bg-surface-3/30"
                    >
                        <span class="text-xl font-bold mb-2 group-hover:text-blue-600 dark:group-hover:text-blue-400 text-gray-800 dark:text-gray-100">Getting Started</span>
                        <p class="text-sm text-gray-500 dark:text-gray-400">Quick guide to your first transcription and project setup.</p>
                    </button>

                    <button 
                        on:click={() => navigateTo('projects')}
                        class="flex flex-col p-6 text-left border border-gray-200 dark:border-gray-700 rounded-xl hover:border-blue-500 dark:hover:border-blue-500 hover:shadow-md transition-all group bg-gray-50/50 dark:bg-surface-3/30"
                    >
                        <span class="text-xl font-bold mb-2 group-hover:text-blue-600 dark:group-hover:text-blue-400 text-gray-800 dark:text-gray-100">Projects</span>
                        <p class="text-sm text-gray-500 dark:text-gray-400">How to create, open, and organize your project workspace.</p>
                    </button>

                    <button 
                        on:click={() => navigateTo('supported-platforms')}
                        class="flex flex-col p-6 text-left border border-gray-200 dark:border-gray-700 rounded-xl hover:border-blue-500 dark:hover:border-blue-500 hover:shadow-md transition-all group bg-gray-50/50 dark:bg-surface-3/30"
                    >
                        <span class="text-xl font-bold mb-2 group-hover:text-blue-600 dark:group-hover:text-blue-400 text-gray-800 dark:text-gray-100">Supported Platforms</span>
                        <p class="text-sm text-gray-500 dark:text-gray-400">Check compatibility and optimized performance guides.</p>
                    </button>
                </div>
              </div>

            {:else if currentPageId === 'about-harvey'}
              <div class="space-y-6">
                <AboutBody />
              </div>

            {:else if currentPageId === 'getting-started'}
              <div class="space-y-6">
                <h3 class="text-2xl font-bold text-gray-900 dark:text-white border-b pb-4 border-gray-100 dark:border-gray-800">Getting Started</h3>
                <ol class="space-y-6 list-none counter-reset-step">
                    <li class="flex items-start space-x-4">
                        <div class="flex-shrink-0 w-8 h-8 rounded-full bg-blue-600 text-white flex items-center justify-center font-bold text-sm">1</div>
                        <div>
                            <h4 class="font-bold text-lg text-gray-800 dark:text-gray-100">Create a Project</h4>
                            <p>Launch the app and select "Create Project". Choose a folder on your disk where all assets will be stored.</p>
                        </div>
                    </li>
                    <li class="flex items-start space-x-4">
                        <div class="flex-shrink-0 w-8 h-8 rounded-full bg-blue-600 text-white flex items-center justify-center font-bold text-sm">2</div>
                        <div>
                            <h4 class="font-bold text-lg text-gray-800 dark:text-gray-100">Import Media</h4>
                            <p>Drag and drop or use the Import menu to add Audio or Video files to your project.</p>
                        </div>
                    </li>
                    <li class="flex items-start space-x-4">
                        <div class="flex-shrink-0 w-8 h-8 rounded-full bg-blue-600 text-white flex items-center justify-center font-bold text-sm">3</div>
                        <div>
                            <h4 class="font-bold text-lg text-gray-800 dark:text-gray-100">Transcription</h4>
                            <p>Head to the Transcription tab, select your file, and hit "Transcribe" to generate your first text.</p>
                        </div>
                    </li>
                </ol>
              </div>

            {:else if currentPageId === 'projects'}
              <div class="space-y-6">
                <h3 class="text-2xl font-bold text-gray-900 dark:text-white border-b pb-4 border-gray-100 dark:border-gray-800">Understanding Projects</h3>
                <p>A Harvey Project is a self-contained folder structure. Moving this folder moves your entire research environment.</p>
                <div class="bg-gray-100 dark:bg-gray-800 p-4 rounded font-mono text-sm overflow-x-auto whitespace-pre">Project Folder/
├── ProjectName.hvy (XML Data)
└── harvey_files/
    ├── Media/ (Audio/Video sources)
    ├── Documents/ (JSON/PDF/MD)
    └── Transcripts/ (Generated text)</div>
              </div>

            {:else if currentPageId === 'supported-platforms'}
              <div class="space-y-6">
                <h3 class="text-2xl font-bold text-gray-900 dark:text-white border-b pb-4 border-gray-100 dark:border-gray-800">Supported Platforms</h3>
                <div class="space-y-4">
                    <div class="p-4 border border-gray-200 dark:border-gray-700 rounded-lg">
                        <span class="font-semibold text-gray-800 dark:text-gray-100">macOS (Apple Silicon)</span>
                    </div>
                    <div class="p-4 border border-gray-200 dark:border-gray-700 rounded-lg">
                        <span class="font-semibold text-gray-800 dark:text-gray-100">macOS (Intel)</span>
                    </div>
                    <div class="p-4 border border-gray-200 dark:border-gray-700 rounded-lg">
                        <span class="font-semibold text-gray-800 dark:text-gray-100">Windows 10/11</span>
                    </div>
                </div>
              </div>

            {:else if currentPageId === 'configure'}
              <div class="space-y-6">
                <h3 class="text-2xl font-bold text-gray-900 dark:text-white border-b pb-4 border-gray-100 dark:border-gray-800">Configure</h3>
                <p>The Configure screen allows you to manage system-level settings, hardware acceleration, and AI models.</p>
                <div class="grid grid-cols-1 md:grid-cols-2 gap-4 mt-6">
                    <button on:click={() => navigateTo('config-app')} class="p-4 text-left border rounded-lg hover:border-blue-500 transition-colors bg-gray-50/50 dark:bg-surface-3/30">
                        <span class="font-bold block mb-1 text-gray-800 dark:text-gray-100">Application</span>
                        <span class="text-xs text-gray-500">Theme and download locations.</span>
                    </button>
                    <button on:click={() => navigateTo('config-transcription')} class="p-4 text-left border rounded-lg hover:border-blue-500 transition-colors bg-gray-50/50 dark:bg-surface-3/30">
                        <span class="font-bold block mb-1 text-gray-800 dark:text-gray-100">Transcription Engine</span>
                        <span class="text-xs text-gray-500">Whisper model management.</span>
                    </button>
                    <button on:click={() => navigateTo('config-diarization')} class="p-4 text-left border rounded-lg hover:border-blue-500 transition-colors bg-gray-50/50 dark:bg-surface-3/30">
                        <span class="font-bold block mb-1 text-gray-800 dark:text-gray-100">Diarization Engine</span>
                        <span class="text-xs text-gray-500">Speaker identification setup.</span>
                    </button>
                    <button on:click={() => navigateTo('config-translation')} class="p-4 text-left border rounded-lg hover:border-blue-500 transition-colors bg-gray-50/50 dark:bg-surface-3/30">
                        <span class="font-bold block mb-1 text-gray-800 dark:text-gray-100">Translation Engine</span>
                        <span class="text-xs text-gray-500">Local translation models.</span>
                    </button>
                    <button on:click={() => navigateTo('config-advanced')} class="p-4 text-left border rounded-lg hover:border-blue-500 transition-colors bg-gray-50/50 dark:bg-surface-3/30">
                        <span class="font-bold block mb-1 text-gray-800 dark:text-gray-100">Advanced Settings</span>
                        <span class="text-xs text-gray-500">Developer tools and logs.</span>
                    </button>
                </div>
              </div>

            {:else if currentPageId === 'config-app'}
              <div class="space-y-6">
                <h3 class="text-2xl font-bold text-gray-900 dark:text-white border-b pb-4 border-gray-100 dark:border-gray-800">Application Settings</h3>
                <ul class="list-disc pl-5 space-y-3 text-base">
                    <li><strong>Theme:</strong> Switch between Light, Dark, or System-defined visual modes.</li>
                    <li><strong>Model Download Location:</strong> Choose where Harvey stores the AI models (Whisper, Translation, etc.) on your disk. You can move existing models to a new location directly from this screen.</li>
                    <li><strong>Required Tools:</strong> Check the status of your Python environment and FFmpeg installation.</li>
                    <li><strong>Hugging Face Integration:</strong> Manage your access token for downloading diarization models.</li>
                </ul>
              </div>

            {:else if currentPageId === 'config-transcription'}
              <div class="space-y-6 text-base">
                <h3 class="text-2xl font-bold text-gray-900 dark:text-white border-b pb-4 border-gray-100 dark:border-gray-800">Transcription Engine</h3>
                <p>Manage the <strong>Whisper.cpp</strong> models used for local transcription.</p>
                <ul class="list-disc pl-5 space-y-3">
                    <li><strong>Available Models:</strong> Download various model sizes (Tiny, Base, Small, Medium, Large). Larger models are more accurate but require more RAM and processing power.</li>
                    <li><strong>Hardware Acceleration:</strong> On macOS (Silicon), Harvey uses Core ML for optimal performance. Windows users can leverage specialized builds for acceleration.</li>
                </ul>
              </div>

            {:else if currentPageId === 'config-diarization'}
              <div class="space-y-6 text-base">
                <h3 class="text-2xl font-bold text-gray-900 dark:text-white border-b pb-4 border-gray-100 dark:border-gray-800">Diarization Engine</h3>
                <p>Speaker identification is powered by <strong>Pyannote Audio</strong> models.</p>
                <ul class="list-disc pl-5 space-y-3">
                    <li><strong>Model Access:</strong> Due to licensing, you must accept the terms on Hugging Face and provide a token in the Application settings to download these models.</li>
                    <li><strong>Functionality:</strong> Once enabled, Harvey can automatically detect when different people are speaking and label them accordingly in the transcript.</li>
                </ul>
              </div>

            {:else if currentPageId === 'config-translation'}
              <div class="space-y-6 text-base">
                <h3 class="text-2xl font-bold text-gray-900 dark:text-white border-b pb-4 border-gray-100 dark:border-gray-800">Translation Engine</h3>
                <p>Local translation is handled by <strong>CTranslate2</strong> using <strong>Helsinki-NLP</strong> and <strong>NLLB</strong> models.</p>
                <ul class="list-disc pl-5 space-y-3">
                    <li><strong>Language Pairs:</strong> Download specific models for the languages you need to translate (e.g., French to English, Spanish to English).</li>
                    <li><strong>Optimization:</strong> Models are automatically optimized for high-speed CPU inference during the download process.</li>
                </ul>
              </div>

            {:else if currentPageId === 'config-advanced'}
              <div class="space-y-6 text-base">
                <h3 class="text-2xl font-bold text-gray-900 dark:text-white border-b pb-4 border-gray-100 dark:border-gray-800">Advanced Settings</h3>
                <p>Tools for troubleshooting and advanced system management.</p>
                <ul class="list-disc pl-5 space-y-3">
                    <li><strong>Developer Logs:</strong> View internal application logs to diagnose issues.</li>
                    <li><strong>Python Management:</strong> Reset or re-initialize the internal Python virtual environment if libraries become corrupted.</li>
                    <li><strong>Cache Management:</strong> Clear temporary files and downloaded model caches.</li>
                </ul>
              </div>

            {:else if currentPageId === 'manage-data'}
              <div class="space-y-6">
                <h3 class="text-2xl font-bold text-gray-900 dark:text-white border-b pb-4 border-gray-100 dark:border-gray-800">Data</h3>
                <p>The Data tab is the central hub for all project assets. This section covers how to organize and work with various file types.</p>
                <div class="grid grid-cols-1 md:grid-cols-2 gap-4 mt-6">
                    <button on:click={() => navigateTo('data-tab')} class="p-4 text-left border rounded-lg hover:border-blue-500 transition-colors bg-gray-50/50 dark:bg-surface-3/30">
                        <span class="font-bold block mb-1 text-gray-800 dark:text-gray-100">Data Tab</span>
                        <span class="text-xs text-gray-500">Navigation, categories, and organization.</span>
                    </button>
                    <button on:click={() => navigateTo('audio')} class="p-4 text-left border rounded-lg hover:border-blue-500 transition-colors bg-gray-50/50 dark:bg-surface-3/30">
                        <span class="font-bold block mb-1 text-gray-800 dark:text-gray-100">Audios</span>
                        <span class="text-xs text-gray-500">Working with sound recordings.</span>
                    </button>
                    <button on:click={() => navigateTo('video')} class="p-4 text-left border rounded-lg hover:border-blue-500 transition-colors bg-gray-50/50 dark:bg-surface-3/30">
                        <span class="font-bold block mb-1 text-gray-800 dark:text-gray-100">Videos</span>
                        <span class="text-xs text-gray-500">Managing visual media.</span>
                    </button>
                    <button on:click={() => navigateTo('documents')} class="p-4 text-left border rounded-lg hover:border-blue-500 transition-colors bg-gray-50/50 dark:bg-surface-3/30">
                        <span class="font-bold block mb-1 text-gray-800 dark:text-gray-100">Documents</span>
                        <span class="text-xs text-gray-500">Text analysis and annotation.</span>
                    </button>
                    <button on:click={() => navigateTo('images')} class="p-4 text-left border rounded-lg hover:border-blue-500 transition-colors bg-gray-50/50 dark:bg-surface-3/30">
                        <span class="font-bold block mb-1 text-gray-800 dark:text-gray-100">Images</span>
                        <span class="text-xs text-gray-500">Visual evidence and screenshots.</span>
                    </button>
                    <button on:click={() => navigateTo('tables')} class="p-4 text-left border rounded-lg hover:border-blue-500 transition-colors bg-gray-50/50 dark:bg-surface-3/30">
                        <span class="font-bold block mb-1 text-gray-800 dark:text-gray-100">Tables</span>
                        <span class="text-xs text-gray-500">CSV and Excel data management.</span>
                    </button>
                    <button on:click={() => navigateTo('transcripts')} class="p-4 text-left border rounded-lg hover:border-blue-500 transition-colors bg-gray-50/50 dark:bg-surface-3/30">
                        <span class="font-bold block mb-1 text-gray-800 dark:text-gray-100">Transcripts</span>
                        <span class="text-xs text-gray-500">Working with external text sources.</span>
                    </button>
                    <button on:click={() => navigateTo('groups')} class="p-4 text-left border rounded-lg hover:border-blue-500 transition-colors bg-gray-50/50 dark:bg-surface-3/30">
                        <span class="font-bold block mb-1 text-gray-800 dark:text-gray-100">Groups</span>
                        <span class="text-xs text-gray-500">Custom organization folders.</span>
                    </button>
                </div>
              </div>

            {:else if currentPageId === 'data-tab'}
              <div class="space-y-6">
                <h3 class="text-2xl font-bold text-gray-900 dark:text-white border-b pb-4 border-gray-100 dark:border-gray-800">Data Tab</h3>
                <p>The Data tab provides a structured view of all your project's assets.</p>
                <ul class="list-disc pl-5 space-y-3">
                    <li><strong>Categories:</strong> Assets are automatically grouped by type (Audios, Videos, Documents, etc.).</li>
                    <li><strong>Context Menus:</strong> Right-click any item to reveal its location on disk, rename it, or delete it.</li>
                    <li><strong>Groups:</strong> Create custom folders to organize related assets across different categories.</li>
                    <li><strong>Search:</strong> Quickly find assets by name using the search bar at the top of the panel.</li>
                </ul>
              </div>

            {:else if currentPageId === 'audio'}
              <div class="space-y-6">
                <h3 class="text-2xl font-bold text-gray-900 dark:text-white border-b pb-4 border-gray-100 dark:border-gray-800">Audios</h3>
                <p>Harvey supports common audio formats including MP3, WAV, M4A, and FLAC.</p>
                <ul class="list-disc pl-5 space-y-3">
                    <li><strong>Playback:</strong> Click an audio file to open it in the Media View. Use the integrated player to seek and control volume.</li>
                    <li><strong>Association:</strong> Audio files are linked to their transcripts. When viewing an audio file, you can see and edit its associated text side-by-side.</li>
                    <li><strong>Trimming:</strong> Use the trim tool to extract specific segments from longer recordings.</li>
                </ul>
              </div>

            {:else if currentPageId === 'video'}
              <div class="space-y-6">
                <h3 class="text-2xl font-bold text-gray-900 dark:text-white border-b pb-4 border-gray-100 dark:border-gray-800">Videos</h3>
                <p>Video support includes MP4, MOV, and MKV formats.</p>
                <ul class="list-disc pl-5 space-y-3">
                    <li><strong>Visual Context:</strong> Play videos directly within the app. The player remains synchronized with the transcript segments.</li>
                    <li><strong>Full Screen:</strong> Double-click the video area to toggle full-screen playback.</li>
                    <li><strong>Screenshots:</strong> Capture specific frames as Image assets for further annotation.</li>
                </ul>
              </div>

            {:else if currentPageId === 'documents'}
              <div class="space-y-6">
                <h3 class="text-2xl font-bold text-gray-900 dark:text-white border-b pb-4 border-gray-100 dark:border-gray-800">Documents</h3>
                <p>Work with text-based assets including Lexical JSON, PDF, Markdown, and TXT files.</p>
                <ul class="list-disc pl-5 space-y-3">
                    <li><strong>Rich Text Editor:</strong> Edit documents with standard formatting tools (Bold, Italic, Lists, etc.).</li>
                    <li><strong>PDF Viewer:</strong> View and annotate PDF documents directly. Annotations are saved as metadata and can be tagged.</li>
                    <li><strong>Highlights:</strong> Select any text to create a highlight. Assign tags to these highlights for cross-project analysis.</li>
                </ul>
              </div>

            {:else if currentPageId === 'images'}
              <div class="space-y-6">
                <h3 class="text-2xl font-bold text-gray-900 dark:text-white border-b pb-4 border-gray-100 dark:border-gray-800">Images</h3>
                <p>Manage visual evidence like photos, diagrams, and video screenshots.</p>
                <ul class="list-disc pl-5 space-y-3">
                    <li><strong>Annotations:</strong> Draw bounding boxes or shapes on images to highlight specific areas.</li>
                    <li><strong>Comments:</strong> Add detailed notes to each image annotation.</li>
                    <li><strong>Export:</strong> Export annotated images for use in reports or presentations.</li>
                </ul>
              </div>

            {:else if currentPageId === 'tables'}
              <div class="space-y-6">
                <h3 class="text-2xl font-bold text-gray-900 dark:text-white border-b pb-4 border-gray-100 dark:border-gray-800">Tables</h3>
                <p>Import and manage structured data from CSV and Excel files.</p>
                <ul class="list-disc pl-5 space-y-3">
                    <li><strong>Data Grid:</strong> View large datasets in a high-performance grid.</li>
                    <li><strong>Headers:</strong> Define custom headers or use the ones from your source file.</li>
                    <li><strong>Cell Highlighting:</strong> Color-code specific cells or rows to mark important data points.</li>
                </ul>
              </div>

            {:else if currentPageId === 'transcripts'}
              <div class="space-y-6">
                <h3 class="text-2xl font-bold text-gray-900 dark:text-white border-b pb-4 border-gray-100 dark:border-gray-800">Transcripts</h3>
                <p>Manage transcripts generated outside of Harvey or imported from Word documents.</p>
                <ul class="list-disc pl-5 space-y-3">
                    <li><strong>Word Import:</strong> Convert `.docx` transcripts into Harvey's interactive format.</li>
                    <li><strong>Attachments:</strong> Manually link imported transcripts to existing audio or video assets in your project.</li>
                    <li><strong>Side-by-Side View:</strong> Open multiple transcripts simultaneously for comparison or translation review.</li>
                </ul>
              </div>

            {:else if currentPageId === 'groups'}
              <div class="space-y-6 text-base leading-relaxed">
                <h3 class="text-2xl font-bold text-gray-900 dark:text-white border-b pb-4 border-gray-100 dark:border-gray-800">Groups</h3>
                <p>Groups are custom organizational units that allow you to cluster related assets regardless of their file type.</p>
                <ul class="list-disc pl-5 space-y-3">
                    <li><strong>Creating Groups:</strong> Use the "Create New > Group" menu or the context menu in the Data tab.</li>
                    <li><strong>Organization:</strong> A group can contain any mix of audio, video, documents, and other assets.</li>
                    <li><strong>Management:</strong> You can rename or delete groups without affecting the actual files on disk.</li>
                </ul>
              </div>

            {:else if currentPageId === 'transcribe'}
              <div class="space-y-6">
                <h3 class="text-2xl font-bold text-gray-900 dark:text-white border-b pb-4 border-gray-100 dark:border-gray-800">Transcription</h3>
                <p>The Transcription tab is the heart of Harvey's media processing capabilities.</p>
                <div class="grid grid-cols-1 md:grid-cols-2 gap-4 mt-6 pb-8">
                    <button on:click={() => navigateTo('transcription-tab')} class="p-4 text-left border rounded-lg hover:border-blue-500 transition-colors bg-gray-50/50 dark:bg-surface-3/30">
                        <span class="font-bold block mb-1 text-gray-800 dark:text-gray-100">Transcription Tab</span>
                        <span class="text-xs text-gray-500">Interface overview and navigation.</span>
                    </button>
                    <button on:click={() => navigateTo('auto-transcription')} class="p-4 text-left border rounded-lg hover:border-blue-500 transition-colors bg-gray-50/50 dark:bg-surface-3/30">
                        <span class="font-bold block mb-1 text-gray-800 dark:text-gray-100">Automatic Transcription</span>
                        <span class="text-xs text-gray-500">AI-powered speech-to-text.</span>
                    </button>
                    <button on:click={() => navigateTo('manual-transcription')} class="p-4 text-left border rounded-lg hover:border-blue-500 transition-colors bg-gray-50/50 dark:bg-surface-3/30">
                        <span class="font-bold block mb-1 text-gray-800 dark:text-gray-100">Manual Transcription</span>
                        <span class="text-xs text-gray-500">Direct text entry and segmentation.</span>
                    </button>
                    <button on:click={() => navigateTo('diarization')} class="p-4 text-left border rounded-lg hover:border-blue-500 transition-colors bg-gray-50/50 dark:bg-surface-3/30">
                        <span class="font-bold block mb-1 text-gray-800 dark:text-gray-100">Diarization</span>
                        <span class="text-xs text-gray-500">Speaker identification.</span>
                    </button>
                    <button on:click={() => navigateTo('translation')} class="p-4 text-left border rounded-lg hover:border-blue-500 transition-colors bg-gray-50/50 dark:bg-surface-3/30">
                        <span class="font-bold block mb-1 text-gray-800 dark:text-gray-100">Translation</span>
                        <span class="text-xs text-gray-500">Local multilingual support.</span>
                    </button>
                    <button on:click={() => navigateTo('media-player')} class="p-4 text-left border rounded-lg hover:border-blue-500 transition-colors bg-gray-50/50 dark:bg-surface-3/30">
                        <span class="font-bold block mb-1 text-gray-800 dark:text-gray-100">Media Player</span>
                        <span class="text-xs text-gray-500">Playback controls and features.</span>
                    </button>
                    <button on:click={() => navigateTo('editing')} class="p-4 text-left border rounded-lg hover:border-blue-500 transition-colors bg-gray-50/50 dark:bg-surface-3/30">
                        <span class="font-bold block mb-1 text-gray-800 dark:text-gray-100">Editing</span>
                        <span class="text-xs text-gray-500">Fixing and refining transcripts.</span>
                    </button>
                    <button on:click={() => navigateTo('waveform')} class="p-4 text-left border rounded-lg hover:border-blue-500 transition-colors bg-gray-50/50 dark:bg-surface-3/30">
                        <span class="font-bold block mb-1 text-gray-800 dark:text-gray-100">Waveform</span>
                        <span class="text-xs text-gray-500">Visual audio representation.</span>
                    </button>
                    <button on:click={() => navigateTo('layout')} class="p-4 text-left border rounded-lg hover:border-blue-500 transition-colors bg-gray-50/50 dark:bg-surface-3/30">
                        <span class="font-bold block mb-1 text-gray-800 dark:text-gray-100">Layout</span>
                        <span class="text-xs text-gray-500">Customizing the workspace.</span>
                    </button>
                    <button on:click={() => navigateTo('shortcuts')} class="p-4 text-left border rounded-lg hover:border-blue-500 transition-colors bg-gray-50/50 dark:bg-surface-3/30">
                        <span class="font-bold block mb-1 text-gray-800 dark:text-gray-100">Shortcuts</span>
                        <span class="text-xs text-gray-500">Keyboard productivity.</span>
                    </button>
                    <button on:click={() => navigateTo('export')} class="p-4 text-left border rounded-lg hover:border-blue-500 transition-colors bg-gray-50/50 dark:bg-surface-3/30">
                        <span class="font-bold block mb-1 text-gray-800 dark:text-gray-100">Export</span>
                        <span class="text-xs text-gray-500">Outputting your results.</span>
                    </button>
                </div>
              </div>

            {:else if currentPageId === 'transcription-tab'}
              <div class="space-y-6">
                <h3 class="text-2xl font-bold text-gray-900 dark:text-white border-b pb-4 border-gray-100 dark:border-gray-800">Transcription Tab</h3>
                <p>This tab is dedicated to media playback and text generation. It features a synchronized environment where the audio/video player, interactive waveform, and transcript editor work in harmony.</p>
                <ul class="list-disc pl-5 space-y-3">
                    <li><strong>Integrated Workflow:</strong> The transcript segments are linked to the media. Selecting a segment jumps the player to that time, and moving the playhead highlights the active segment.</li>
                    <li><strong>Standardized View:</strong> Segments use a fixed two-row layout (Metadata then Text) to maximize horizontal space for both short and long utterances.</li>
                    <li><strong>Interactive Panels:</strong> All panels are resizable and collapsible to optimize your workspace for different tasks like review or deep editing.</li>
                </ul>
              </div>

            {:else if currentPageId === 'auto-transcription'}
              <div class="space-y-6">
                <h3 class="text-2xl font-bold text-gray-900 dark:text-white border-b pb-4 border-gray-100 dark:border-gray-800">Automatic Transcription</h3>
                <p>Leverage state-of-the-art <strong>Whisper</strong> models to convert speech to text automatically and locally.</p>
                <ul class="list-disc pl-5 space-y-3">
                    <li><strong>System Checks:</strong> Harvey automatically verifies your Python environment and hardware acceleration (like Core ML) before starting to ensure peak performance.</li>
                    <li><strong>Model Selection:</strong> Choose a model size that matches your accuracy needs. Larger models (e.g., Medium, Large) provide better results for complex audio but require more RAM.</li>
                    <li><strong>Language Detection:</strong> Set a specific language for best results, or use "Auto Detect" for multilingual files.</li>
                </ul>
              </div>

            {:else if currentPageId === 'manual-transcription'}
              <div class="space-y-6">
                <h3 class="text-2xl font-bold text-gray-900 dark:text-white border-b pb-4 border-gray-100 dark:border-gray-800">Manual Transcription</h3>
                <p>For high-precision work or difficult audio, you can transcribe manually using Harvey's specialized tools.</p>
                <ul class="list-disc pl-5 space-y-3">
                    <li><strong>Smart Initialization:</strong> When setting up a manual transcript, the <strong>Number of Segments</strong> and <strong>Duration</strong> are bidirectionally linked. Adjusting one automatically calculates the other based on the total media length.</li>
                    <li><strong>Segmentation:</strong> Create custom time-stamped segments manually. The system ensures segments are perfectly contiguous.</li>
                    <li><strong>Direct Entry:</strong> Type directly into the segments while controlling playback with global keyboard shortcuts.</li>
                    <li><strong>Flexibility:</strong> Combine manual entry with automatic results by editing existing segments at any time.</li>
                </ul>
              </div>

            {:else if currentPageId === 'diarization'}
              <div class="space-y-6">
                <h3 class="text-2xl font-bold text-gray-900 dark:text-white border-b pb-4 border-gray-100 dark:border-gray-800">Diarization</h3>
                <p>Automatically identify and label different speakers in an audio recording.</p>
                <ul class="list-disc pl-5 space-y-3">
                    <li><strong>Speaker Labels:</strong> AI assigns consistent labels (e.g., Speaker 1, Speaker 2) across the transcript.</li>
                    <li><strong>Renaming:</strong> Double-click any speaker label to rename it throughout the entire document.</li>
                    <li><strong>Configuration:</strong> Requires the Pyannote diarization model, which can be enabled in the Configure screen.</li>
                </ul>
              </div>

            {:else if currentPageId === 'translation'}
              <div class="space-y-6 text-base leading-relaxed">
                <h3 class="text-2xl font-bold text-gray-900 dark:text-white border-b pb-4 border-gray-100 dark:border-gray-800">Translation</h3>
                <p>Harvey supports local machine translation using the <strong>Helsinki-NLP</strong> and <strong>NLLB</strong> (No Language Left Behind) models. Translation is optimized using <strong>CTranslate2</strong> for high performance on standard CPUs.</p>
                
                <h4 class="font-bold text-lg mt-6 mb-2 text-gray-800 dark:text-gray-100">Core Features</h4>
                <ul class="list-disc pl-5 space-y-3">
                    <li><strong>Privacy:</strong> All translations happen locally. Your sensitive data is never sent to external servers.</li>
                    <li><strong>Transcript Translation:</strong> Translate an entire transcript while maintaining the original timestamps and speaker labels.</li>
                    <li><strong>Side-by-Side View:</strong> View the original and translated versions together in the editor for easy comparison.</li>
                    <li><strong>Document Translation:</strong> Quickly translate your research notes and other documents in the Data tab.</li>
                </ul>

                <div class="p-4 bg-yellow-50 dark:bg-yellow-900/20 rounded-lg border border-yellow-100 dark:border-yellow-800 mt-6">
                    <strong class="text-yellow-900 dark:text-yellow-300">Requirement:</strong> You must download the specific translation models for your language pair in the <strong>Configure</strong> screen before using this feature.
                </div>
              </div>

            {:else if currentPageId === 'media-player'}
              <div class="space-y-6">
                <h3 class="text-2xl font-bold text-gray-900 dark:text-white border-b pb-4 border-gray-100 dark:border-gray-800">Media Player</h3>
                <p>The integrated media player is optimized for transcription workflows.</p>
                <ul class="list-disc pl-5 space-y-3">
                    <li><strong>Synchronization:</strong> Clicking any text segment jumps the player to that exact moment in the audio/video.</li>
                    <li><strong>Playback Speed:</strong> Adjust speed from 0.5x to 2.0x to match your typing pace.</li>
                    <li><strong>Reliable Trimming:</strong> Use the trim tool to extract clips from your media assets. The process is optimized to handle large files reliably.</li>
                </ul>
              </div>

            {:else if currentPageId === 'editing'}
              <div class="space-y-6 text-base leading-relaxed">
                <h3 class="text-2xl font-bold text-gray-900 dark:text-white border-b pb-4 border-gray-100 dark:border-gray-800">Editing</h3>
                <p>Refine your transcripts with powerful, time-aware editing tools.</p>
                <ul class="list-disc pl-5 space-y-3">
                    <li><strong>Edit Mode:</strong> Toggle edit mode using the toolbar button or <kbd class="px-1.5 py-0.5 bg-gray-100 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded text-xs">{modKeyName} + E</kbd>.</li>
                    <li><strong>Gapless Continuity:</strong> When you adjust the start or end time of a segment, the adjacent segments are automatically updated to prevent gaps or overlaps, ensuring a perfect timeline.</li>
                    <li><strong>Standardized Layout:</strong> Each segment uses a clear two-row layout: Metadata (Speaker & Time) at the top, and formatted text below for maximum clarity.</li>
                    <li><strong>Formatting:</strong> Apply bold, italic, or underline styles to emphasize specific parts of the text.</li>
                </ul>
              </div>

            {:else if currentPageId === 'waveform'}
              <div class="space-y-6">
                <h3 class="text-2xl font-bold text-gray-900 dark:text-white border-b pb-4 border-gray-100 dark:border-gray-800">Waveform</h3>
                <p>The interactive waveform provides a visual representation of your audio and segment timing.</p>
                <ul class="list-disc pl-5 space-y-3">
                    <li><strong>Interactive Handles:</strong> In the vertical waveform view, you can drag the top and bottom handles of any segment block to precisely adjust its timing.</li>
                    <li><strong>Visual Cues:</strong> See pauses and loud sections at a glance, helping you identify speaker turns and natural segment boundaries.</li>
                    <li><strong>Navigation:</strong> Click anywhere on the waveform to seek. The red playhead shows the current position.</li>
                    <li><strong>Zooming:</strong> Use the zoom controls or <kbd class="px-1.5 py-0.5 bg-gray-100 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded text-xs">{modKeyName} + Scroll</kbd> to inspect fine audio details.</li>
                </ul>
              </div>

            {:else if currentPageId === 'layout'}
              <div class="space-y-6 text-base leading-relaxed">
                <h3 class="text-2xl font-bold text-gray-900 dark:text-white border-b pb-4 border-gray-100 dark:border-gray-800">Layout</h3>
                <p>Customize the Transcription workspace to suit your preferences.</p>
                <ul class="list-disc pl-5 space-y-3">
                    <li><strong>Two-Row Segments:</strong> The transcript uses a standardized two-row structure (Metadata followed by Text) to provide more horizontal space for long utterances.</li>
                    <li><strong>Panel Resizing:</strong> Drag the dividers between the file browser, editor, and info panel.</li>
                    <li><strong>Responsive Headers:</strong> The waveform headers are optimized to remain functional even when panels are narrow.</li>
                </ul>
              </div>

            {:else if currentPageId === 'shortcuts'}
              <div class="space-y-6">
                <h3 class="text-2xl font-bold text-gray-900 dark:text-white border-b pb-4 border-gray-100 dark:border-gray-800">Shortcuts</h3>
                <p>Master these platform-aware shortcuts to significantly speed up your workflow.</p>
                <div class="grid grid-cols-2 gap-4 max-w-lg mt-4">
                    <div class="font-mono bg-gray-100 dark:bg-gray-800 px-2 py-1 rounded text-xs">{modKeyName} + E</div>
                    <div class="text-sm">Toggle Edit Mode</div>
                    <div class="font-mono bg-gray-100 dark:bg-gray-800 px-2 py-1 rounded text-xs">{modKeyName} + S</div>
                    <div class="text-sm">Save Transcript</div>
                    <div class="font-mono bg-gray-100 dark:bg-gray-800 px-2 py-1 rounded text-xs">F8</div>
                    <div class="text-sm">Play / Pause Media</div>
                    <div class="font-mono bg-gray-100 dark:bg-gray-800 px-2 py-1 rounded text-xs">F7 / F9</div>
                    <div class="text-sm">Rewind / Forward 5s</div>
                    <div class="font-mono bg-gray-100 dark:bg-gray-800 px-2 py-1 rounded text-xs">{modKeyName} + ↑ / ↓</div>
                    <div class="text-sm">Previous / Next Segment</div>
                </div>
              </div>

            {:else if currentPageId === 'export'}
              <div class="space-y-6">
                <h3 class="text-2xl font-bold text-gray-900 dark:text-white border-b pb-4 border-gray-100 dark:border-gray-800">Export</h3>
                <p>Share your work by exporting transcripts in various formats.</p>
                <ul class="list-disc pl-5 space-y-3 text-base">
                    <li><strong>Standard Formats:</strong> Export to Microsoft Word (.docx), Markdown (.md), or Plain Text (.txt).</li>
                    <li><strong>Subtitles:</strong> Generate timed subtitle files in SRT, VTT, or ASS formats.</li>
                    <li><strong>Options:</strong> Choose which elements to include, such as speaker names and timestamps.</li>
                </ul>
              </div>

            {:else if currentPageId === 'tags'}
              <div class="space-y-6">
                <h3 class="text-2xl font-bold text-gray-900 dark:text-white border-b pb-4 border-gray-100 dark:border-gray-800">Tags</h3>
                <p>The Tags system is a powerful tool for qualitative analysis across your entire project.</p>
                <div class="grid grid-cols-1 md:grid-cols-2 gap-4 mt-6">
                    <button on:click={() => navigateTo('tags-tab')} class="p-4 text-left border rounded-lg hover:border-blue-500 transition-colors bg-gray-50/50 dark:bg-surface-3/30">
                        <span class="font-bold block mb-1 text-gray-800 dark:text-gray-100">Tags Tab</span>
                        <span class="text-xs text-gray-500">Interface and tag management.</span>
                    </button>
                    <button on:click={() => navigateTo('highlights')} class="p-4 text-left border rounded-lg hover:border-blue-500 transition-colors bg-gray-50/50 dark:bg-surface-3/30">
                        <span class="font-bold block mb-1 text-gray-800 dark:text-gray-100">Highlights</span>
                        <span class="text-xs text-gray-500">Creating and assigning tags.</span>
                    </button>
                    <button on:click={() => navigateTo('tag-groups')} class="p-4 text-left border rounded-lg hover:border-blue-500 transition-colors bg-gray-50/50 dark:bg-surface-3/30">
                        <span class="font-bold block mb-1 text-gray-800 dark:text-gray-100">Tag Groups</span>
                        <span class="text-xs text-gray-500">Thematic organization.</span>
                    </button>
                </div>
              </div>

            {:else if currentPageId === 'tags-tab'}
              <div class="space-y-6">
                <h3 class="text-2xl font-bold text-gray-900 dark:text-white border-b pb-4 border-gray-100 dark:border-gray-800">Tags Tab</h3>
                <p>The Tags tab provides a birds-eye view of your qualitative coding schema.</p>
                <ul class="list-disc pl-5 space-y-3">
                    <li><strong>All Tags:</strong> View a flat or grouped list of every tag defined in the project.</li>
                    <li><strong>Frequency:</strong> See how many times each tag has been applied across all documents and transcripts.</li>
                    <li><strong>Management:</strong> Add, rename, or delete tags. Renaming a tag updates it globally across the project.</li>
                </ul>
              </div>

            {:else if currentPageId === 'highlights'}
              <div class="space-y-6">
                <h3 class="text-2xl font-bold text-gray-900 dark:text-white border-b pb-4 border-gray-100 dark:border-gray-800">Highlights</h3>
                <p>Highlights are the primary way to apply your analytical framework to your data.</p>
                <ul class="list-disc pl-5 space-y-3">
                    <li><strong>Creation:</strong> Select any text in a document or transcript. A floating menu will appear allowing you to select a tag.</li>
                    <li><strong>Persistence:</strong> Highlights are saved as project metadata. Clicking a highlight in the Tags tab jumps you to its exact location.</li>
                    <li><strong>Multi-tagging:</strong> You can apply multiple tags to the same snippet of text.</li>
                </ul>
              </div>

            {:else if currentPageId === 'tag-groups'}
              <div class="space-y-6">
                <h3 class="text-2xl font-bold text-gray-900 dark:text-white border-b pb-4 border-gray-100 dark:border-gray-800">Tag Groups</h3>
                <p>Organize your tags into thematic categories for more complex analysis.</p>
                <ul class="list-disc pl-5 space-y-3">
                    <li><strong>Hierarchy:</strong> Group related tags (e.g., "Emotions", "Interactional Patterns") to keep your coding schema organized.</li>
                    <li><strong>Filtering:</strong> Use groups to filter your view in the Tags tab or when exporting highlight reports.</li>
                    <li><strong>Flexibility:</strong> Move tags between groups easily by dragging or using the tag options menu.</li>
                </ul>
              </div>

            {:else if currentPageId === 'annotate-page'}
              <div class="space-y-6">
                <h3 class="text-2xl font-bold text-gray-900 dark:text-white border-b pb-4 border-gray-100 dark:border-gray-800">Annotate</h3>
                <p>Annotation is the core process of adding analytical value to your project data. Harvey supports a wide range of annotation types across different media and document formats.</p>
                <div class="grid grid-cols-1 md:grid-cols-2 gap-4 mt-6">
                    <button on:click={() => navigateTo('media-annotations')} class="p-4 text-left border rounded-lg hover:border-blue-500 transition-colors bg-gray-50/50 dark:bg-surface-3/30">
                        <span class="font-bold block mb-1 text-gray-800 dark:text-gray-100">Media Annotations</span>
                        <span class="text-xs text-gray-500">Annotating audio and video segments.</span>
                    </button>
                    <button on:click={() => navigateTo('document-annotations')} class="p-4 text-left border rounded-lg hover:border-blue-500 transition-colors bg-gray-50/50 dark:bg-surface-3/30">
                        <span class="font-bold block mb-1 text-gray-800 dark:text-gray-100">Document Annotations</span>
                        <span class="text-xs text-gray-500">Working with text documents.</span>
                    </button>
                    <button on:click={() => navigateTo('pdf-annotations')} class="p-4 text-left border rounded-lg hover:border-blue-500 transition-colors bg-gray-50/50 dark:bg-surface-3/30">
                        <span class="font-bold block mb-1 text-gray-800 dark:text-gray-100">PDF Annotations</span>
                        <span class="text-xs text-gray-500">Visual highlights on PDFs.</span>
                    </button>
                    <button on:click={() => navigateTo('image-annotations')} class="p-4 text-left border rounded-lg hover:border-blue-500 transition-colors bg-gray-50/50 dark:bg-surface-3/30">
                        <span class="font-bold block mb-1 text-gray-800 dark:text-gray-100">Image Annotations</span>
                        <span class="text-xs text-gray-500">Geometric bounding boxes.</span>
                    </button>
                    <button on:click={() => navigateTo('table-annotations')} class="p-4 text-left border rounded-lg hover:border-blue-500 transition-colors bg-gray-50/50 dark:bg-surface-3/30">
                        <span class="font-bold block mb-1 text-gray-800 dark:text-gray-100">Table Annotations</span>
                        <span class="text-xs text-gray-500">Categorizing tabular data.</span>
                    </button>
                    <button on:click={() => navigateTo('transcript-annotations')} class="p-4 text-left border rounded-lg hover:border-blue-500 transition-colors bg-gray-50/50 dark:bg-surface-3/30">
                        <span class="font-bold block mb-1 text-gray-800 dark:text-gray-100">Transcript Annotations</span>
                        <span class="text-xs text-gray-500">Interpreting dialogue.</span>
                    </button>
                    <button on:click={() => navigateTo('highlights-panel')} class="p-4 text-left border rounded-lg hover:border-blue-500 transition-colors bg-gray-50/50 dark:bg-surface-3/30">
                        <span class="font-bold block mb-1 text-gray-800 dark:text-gray-100">Highlights Panel</span>
                        <span class="text-xs text-gray-500">Reviewing all annotations.</span>
                    </button>
                </div>
              </div>

            {:else if currentPageId === 'media-annotations'}
              <div class="space-y-6 text-base">
                <h3 class="text-2xl font-bold text-gray-900 dark:text-white border-b pb-4 border-gray-100 dark:border-gray-800">Media Annotations</h3>
                <p>Annotate audio and video files by creating time-stamped segments and attaching notes or tags to them.</p>
                <ul class="list-disc pl-5 space-y-3">
                    <li><strong>Segment Notes:</strong> Add rich text descriptions to any media segment.</li>
                    <li><strong>Temporal Tagging:</strong> Link tags directly to specific moments in time.</li>
                    <li><strong>Media Context:</strong> Annotations remain synchronized with the playback, allowing you to jump straight to the source material.</li>
                </ul>
              </div>

            {:else if currentPageId === 'document-annotations'}
              <div class="space-y-6 text-base">
                <h3 class="text-2xl font-bold text-gray-900 dark:text-white border-b pb-4 border-gray-100 dark:border-gray-800">Document Annotations</h3>
                <p>Annotate text-based documents using the integrated rich text editor.</p>
                <ul class="list-disc pl-5 space-y-3">
                    <li><strong>In-line Comments:</strong> Add notes to specific text selections.</li>
                    <li><strong>Text Highlights:</strong> Use color-coded highlights to categorize different themes.</li>
                    <li><strong>Cross-Referencing:</strong> Create links between different parts of a document or even between different files.</li>
                </ul>
              </div>

            {:else if currentPageId === 'pdf-annotations'}
              <div class="space-y-6 text-base">
                <h3 class="text-2xl font-bold text-gray-900 dark:text-white border-b pb-4 border-gray-100 dark:border-gray-800">PDF Annotations</h3>
                <p>PDF documents can be visually annotated using standard tools.</p>
                <ul class="list-disc pl-5 space-y-3">
                    <li><strong>Visual Highlighting:</strong> Select and highlight text directly on the PDF page.</li>
                    <li><strong>Sticky Notes:</strong> Place comments anywhere on the document layout.</li>
                    <li><strong>Integration:</strong> All PDF highlights are searchable and appear in the project-wide highlights panel.</li>
                </ul>
              </div>

            {:else if currentPageId === 'image-annotations'}
              <div class="space-y-6 text-base">
                <h3 class="text-2xl font-bold text-gray-900 dark:text-white border-b pb-4 border-gray-100 dark:border-gray-800">Image Annotations</h3>
                <p>Work with visual evidence by annotating image files.</p>
                <ul class="list-disc pl-5 space-y-3">
                    <li><strong>Bounding Boxes:</strong> Draw rectangles around specific areas of interest in an image.</li>
                    <li><strong>Geometric Shapes:</strong> Use circles, lines, or polygons for precise visual marking.</li>
                    <li><strong>Annotorious:</strong> Harvey integrates with powerful image annotation libraries to provide professional-grade tools.</li>
                </ul>
              </div>

            {:else if currentPageId === 'table-annotations'}
              <div class="space-y-6 text-base">
                <h3 class="text-2xl font-bold text-gray-900 dark:text-white border-b pb-4 border-gray-100 dark:border-gray-800">Table Annotations</h3>
                <p>Annotate structured data in CSV or Excel tables.</p>
                <ul class="list-disc pl-5 space-y-3">
                    <li><strong>Cell Tagging:</strong> Apply tags to individual cells or entire rows.</li>
                    <li><strong>Data Interpretation:</strong> Add comments to explain specific data points or outliers.</li>
                    <li><strong>Visual Cues:</strong> Color-code rows based on applied tags for quick visual analysis.</li>
                </ul>
              </div>

            {:else if currentPageId === 'transcript-annotations'}
              <div class="space-y-6 text-base">
                <h3 class="text-2xl font-bold text-gray-900 dark:text-white border-b pb-4 border-gray-100 dark:border-gray-800">Transcript Annotations</h3>
                <p>Interpretation of dialogue and social interaction.</p>
                <ul class="list-disc pl-5 space-y-3">
                    <li><strong>Interactional Tagging:</strong> Apply specialized tags to capture conversational patterns.</li>
                    <li><strong>Speaker Context:</strong> Annotate based on who is speaking and the social dynamics at play.</li>
                    <li><strong>Synchronization:</strong> Transcript annotations are linked to the source media for immediate verification.</li>
                </ul>
              </div>

            {:else if currentPageId === 'highlights-panel'}
              <div class="space-y-6 text-base">
                <h3 class="text-2xl font-bold text-gray-900 dark:text-white border-b pb-4 border-gray-100 dark:border-gray-800">Highlights Panel</h3>
                <p>The centralized hub for reviewing and managing all your project annotations.</p>
                <ul class="list-disc pl-5 space-y-3">
                    <li><strong>Global Review:</strong> See all highlights from across every document, transcript, and media file in one place.</li>
                    <li><strong>Advanced Filtering:</strong> Filter by tag, file type, or creator to focus on specific themes.</li>
                    <li><strong>Navigation:</strong> Click any highlight to jump directly to its location in the source file.</li>
                </ul>
              </div>

            {:else if currentPageId === 'translate-page'}
              <div class="space-y-6">
                <h3 class="text-2xl font-bold text-gray-900 dark:text-white border-b pb-4 border-gray-100 dark:border-gray-800">Translate</h3>
                <p>The Translate section covers Harvey's offline machine translation capabilities.</p>
                <div class="grid grid-cols-1 md:grid-cols-2 gap-4 mt-6">
                    <button on:click={() => navigateTo('translate-transcript')} class="p-4 text-left border rounded-lg hover:border-blue-500 transition-colors bg-gray-50/50 dark:bg-surface-3/30">
                        <span class="font-bold block mb-1 text-gray-800 dark:text-gray-100">Translating Transcript</span>
                        <span class="text-xs text-gray-500">Multilingual audio support.</span>
                    </button>
                    <button on:click={() => navigateTo('translate-document')} class="p-4 text-left border rounded-lg hover:border-blue-500 transition-colors bg-gray-50/50 dark:bg-surface-3/30">
                        <span class="font-bold block mb-1 text-gray-800 dark:text-gray-100">Translating Document</span>
                        <span class="text-xs text-gray-500">Converting research notes.</span>
                    </button>
                    <button on:click={() => navigateTo('translate-models')} class="p-4 text-left border rounded-lg hover:border-blue-500 transition-colors bg-gray-50/50 dark:bg-surface-3/30">
                        <span class="font-bold block mb-1 text-gray-800 dark:text-gray-100">Translation Models</span>
                        <span class="text-xs text-gray-500">Managing Helsinki and NLLB.</span>
                    </button>
                </div>
              </div>

            {:else if currentPageId === 'translate-transcript'}
              <div class="space-y-6 text-base">
                <h3 class="text-2xl font-bold text-gray-900 dark:text-white border-b pb-4 border-gray-100 dark:border-gray-800">Translating Transcript</h3>
                <p>Translate your generated transcripts while preserving all metadata.</p>
                <ul class="list-disc pl-5 space-y-3">
                    <li><strong>Structural Preservation:</strong> Translation maintains exact timestamps and speaker associations.</li>
                    <li><strong>Workflow:</strong> Open a transcript in the Transcription tab, click the Translate button, and select your target language.</li>
                    <li><strong>Review:</strong> Toggle between original and translated versions to verify accuracy.</li>
                </ul>
              </div>

            {:else if currentPageId === 'translate-document'}
              <div class="space-y-6 text-base">
                <h3 class="text-2xl font-bold text-gray-900 dark:text-white border-b pb-4 border-gray-100 dark:border-gray-800">Translating Document</h3>
                <p>Convert your documents into other languages directly within the Data tab.</p>
                <ul class="list-disc pl-5 space-y-3">
                    <li><strong>Supported Formats:</strong> Works with Lexical JSON, Markdown, and plain text files.</li>
                    <li><strong>Batch Translation:</strong> Select multiple documents in the Data list to translate them in a single operation.</li>
                    <li><strong>Output:</strong> Translated documents are saved as new assets in your project, linked to the originals.</li>
                </ul>
              </div>

            {:else if currentPageId === 'translate-models'}
              <div class="space-y-6 text-base leading-relaxed">
                <h3 class="text-2xl font-bold text-gray-900 dark:text-white border-b pb-4 border-gray-100 dark:border-gray-800">Translation Models</h3>
                <p>Harvey uses specialized AI models for offline translation.</p>
                <ul class="list-disc pl-5 space-y-3">
                    <li><strong>Helsinki-NLP:</strong> Highly efficient models for specific language pairs.</li>
                    <li><strong>NLLB (No Language Left Behind):</strong> Facebook's universal model for translating between 200+ languages.</li>
                    <li><strong>Management:</strong> Use the Translation Engine settings in the Configure screen to download and optimize these models for your CPU.</li>
                </ul>
              </div>

            {:else if currentPageId === 'report-issue'}
              <div class="space-y-6">
                <h3 class="text-2xl font-bold text-gray-900 dark:text-white border-b pb-4 border-gray-100 dark:border-gray-800">Report Issue</h3>
                
                <section class="space-y-4">
                    <p>Encountered a bug or have a feature request? We use <strong>GitHub Issues</strong> to track and manage all feedback. Reporting an issue helps us make Harvey better for everyone.</p>
                    
                    <div class="bg-gray-50 dark:bg-surface-3 border border-gray-200 dark:border-gray-700 p-6 rounded-xl">
                        <h4 class="font-bold text-gray-900 dark:text-white mb-3 text-sm">How to report on GitHub:</h4>
                        <ol class="list-decimal pl-5 space-y-3 text-sm">
                            <li>Visit the <strong>Harvey Issues</strong> page: <a href="https://github.com/Ethnomethodology/harvey/issues" target="_blank" class="text-blue-600 dark:text-blue-400 hover:underline">github.com/Ethnomethodology/harvey/issues</a></li>
                            <li>Sign in to your GitHub account (or create one for free).</li>
                            <li>Click the green <span class="bg-green-600 text-white px-2 py-0.5 rounded text-xs font-bold">New Issue</span> button.</li>
                            <li>Choose a template (Bug report or Feature request).</li>
                            <li>Provide a clear title and describe the problem in detail. If it's a bug, include the steps to reproduce it.</li>
                        </ol>
                    </div>
                </section>
              </div>
            {/if}

          </div>

          <!-- Inner Page Footer (Navigation) -->
          <div class="flex-shrink-0 px-8 py-4 border-t border-gray-200 dark:border-border flex justify-between items-center bg-gray-50 dark:bg-dark-bg-secondary h-14">
            <button 
                class="flex items-center space-x-1 text-sm font-medium text-gray-600 dark:text-gray-400 hover:text-blue-600 dark:hover:text-blue-400 transition-colors disabled:opacity-0"
                on:click={goToPrev}
                disabled={!prevPage}
            >
                <span class="flex-shrink-0">{@html ICON_PREV}</span>
                <span>Previous: {prevPage?.label || ''}</span>
            </button>

            <button 
                class="flex items-center space-x-1 text-sm font-medium text-gray-600 dark:text-gray-400 hover:text-blue-600 dark:hover:text-blue-400 transition-colors disabled:opacity-0"
                on:click={goToNext}
                disabled={!nextPage}
            >
                <span>Next: {nextPage?.label || ''}</span>
                <span class="flex-shrink-0">{@html ICON_NEXT}</span>
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
{/if}

<style>
  /* Custom scrollbar for Webkit browsers */
  .overflow-y-auto::-webkit-scrollbar {
    width: 6px;
  }
  .overflow-y-auto::-webkit-scrollbar-track {
    background: transparent;
  }
  .overflow-y-auto::-webkit-scrollbar-thumb {
    background-color: rgba(156, 163, 175, 0.3);
    border-radius: 10px;
  }
  .overflow-y-auto::-webkit-scrollbar-thumb:hover {
    background-color: rgba(156, 163, 175, 0.5);
  }

  kbd {
    box-shadow: 0 1px 0 rgba(0,0,0,0.2), inset 0 0 0 1px #fff;
  }
  .dark kbd {
    box-shadow: 0 1px 0 rgba(0,0,0,0.5), inset 0 0 0 1px #444;
  }
</style>
