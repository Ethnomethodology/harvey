<!-- src/lib/components/modals/HelpModal.svelte -->
<script>
  import { createEventDispatcher, onMount } from 'svelte';
  import AboutBody from '$lib/components/shared/AboutBody.svelte';

  export let showModal = false;

  const dispatch = createEventDispatcher();

  // Define the flat sequence of all pages using exact UI strings
  const allPages = [
    { id: 'overview', label: 'Help Center', sidebarId: 'overview' },
    { id: 'about-harvey', label: 'About Harvey', sidebarId: 'overview' },
    { id: 'getting-started', label: 'Getting Started', sidebarId: 'overview' },
    { id: 'projects', label: 'Projects', sidebarId: 'overview' },
    { id: 'supported-platforms', label: 'Supported Platforms', sidebarId: 'overview' },
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
    { id: 'configure', label: 'Configure', sidebarId: 'configure' },
    { id: 'tags', label: 'Tags', sidebarId: 'tags' },
    { id: 'report-issue', label: 'Report Issue', sidebarId: 'report-issue' }
  ];

  // Sidebar tabs (only top-level sections)
  const sidebarTabs = [
    { id: 'overview', label: 'Overview' },
    { id: 'manage-data', label: 'Data' },
    { id: 'transcribe', label: 'Transcription' },
    { id: 'configure', label: 'Configure' },
    { id: 'tags', label: 'Tags' },
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
        <h2 id="help-modal-title" class="text-xl font-bold text-gray-800 dark:text-white">Help Center</h2>
        
        <div class="flex items-center space-x-4 flex-grow justify-end pr-4">
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
          <div class="flex-shrink-0 px-8 py-3 border-b border-gray-100 dark:border-gray-800 flex justify-between items-center bg-white dark:bg-surface-2 h-12">
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
                    <div class="flex justify-between items-center p-4 border border-gray-200 dark:border-gray-700 rounded-lg">
                        <span class="font-semibold text-gray-800 dark:text-gray-100">macOS (Silicon)</span>
                        <span class="text-green-500 text-sm font-medium">Optimal Performance</span>
                    </div>
                    <div class="flex justify-between items-center p-4 border border-gray-200 dark:border-gray-700 rounded-lg">
                        <span class="font-semibold text-gray-800 dark:text-gray-100">macOS (Intel)</span>
                        <span class="text-yellow-500 text-sm font-medium">Supported</span>
                    </div>
                    <div class="flex justify-between items-center p-4 border border-gray-200 dark:border-gray-700 rounded-lg">
                        <span class="font-semibold text-gray-800 dark:text-gray-100">Windows 10/11</span>
                        <span class="text-green-500 text-sm font-medium">Fully Supported</span>
                    </div>
                </div>
              </div>

            {:else if currentPageId === 'configure'}
              <div class="space-y-6">
                <h3 class="text-2xl font-bold text-gray-900 dark:text-white border-b pb-4 border-gray-100 dark:border-gray-800">Configure</h3>
                <p>Access the <strong>Configure</strong> screen via the cog icon in the sidebar or from the Welcome screen.</p>
                <ul class="list-disc pl-5 space-y-2">
                    <li><strong>Models:</strong> Download Whisper models (Tiny, Base, Small, Medium, Large).</li>
                    <li><strong>Python:</strong> Ensure the virtual environment is correctly initialized.</li>
                    <li><strong>Theme:</strong> Toggle between Light, Dark, and System modes.</li>
                </ul>
              </div>

            {:else if currentPageId === 'transcribe'}
              <div class="space-y-6">
                <h3 class="text-2xl font-bold text-gray-900 dark:text-white border-b pb-4 border-gray-100 dark:border-gray-800">Transcription</h3>
                <p>The Transcription tab is where the heavy lifting happens.</p>
                <ul class="list-disc pl-5 space-y-2">
                    <li><strong>Waveform:</strong> Visual seek bar for your audio.</li>
                    <li><strong>Edit Mode:</strong> Click "Edit" or press <kbd class="px-1.5 py-0.5 bg-gray-100 dark:bg-gray-700 border border-gray-300 dark:border-gray-600 rounded text-xs">Ctrl + E</kbd> to fix text.</li>
                    <li><strong>Speaker Labels:</strong> Double-click speaker names to rename them globally.</li>
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
              <div class="space-y-6">
                <h3 class="text-2xl font-bold text-gray-900 dark:text-white border-b pb-4 border-gray-100 dark:border-gray-800">Groups</h3>
                <p>Groups are custom organizational units that allow you to cluster related assets regardless of their file type.</p>
                <ul class="list-disc pl-5 space-y-3">
                    <li><strong>Creating Groups:</strong> Use the "Create New > Group" menu or the context menu in the Data tab.</li>
                    <li><strong>Organization:</strong> A group can contain any mix of audio, video, documents, and other assets.</li>
                    <li><strong>Management:</strong> You can rename or delete groups without affecting the actual files on disk.</li>
                </ul>
              </div>

            {:else if currentPageId === 'tags'}
              <div class="space-y-6">
                <h3 class="text-2xl font-bold text-gray-900 dark:text-white border-b pb-4 border-gray-100 dark:border-gray-800">Tags</h3>
                <p>Tags allow you to categorize snippets of text across all your documents and transcripts.</p>
                <ul class="list-disc pl-5 space-y-2">
                    <li><strong>Creating Tags:</strong> In the Tags tab, click the options button to add new tags or groups.</li>
                    <li><strong>Assigning Tags:</strong> Simply select text in any editor and choose a tag from the highlight menu.</li>
                    <li><strong>Tag Groups:</strong> Organize related tags into groups for better analysis.</li>
                </ul>
              </div>

            {:else if currentPageId === 'report-issue'}
              <div class="space-y-6">
                <h3 class="text-2xl font-bold text-gray-900 dark:text-white border-b pb-4 border-gray-100 dark:border-gray-800">Report Issue</h3>
                
                <section class="space-y-4">
                    <p>Encountered a bug or have a feature request? We use <strong>GitHub Issues</strong> to track and manage all feedback. Reporting an issue helps us make Harvey better for everyone.</p>
                    
                    <div class="bg-gray-50 dark:bg-surface-3 border border-gray-200 dark:border-gray-700 p-6 rounded-xl">
                        <h4 class="font-bold text-gray-900 dark:text-white mb-3">How to report on GitHub:</h4>
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