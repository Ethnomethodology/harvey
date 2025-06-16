<script>
    import { onMount, onDestroy } from 'svelte';
    import { project, prepareDocumentView, prepareImportedTranscriptView, prepareMediaNoteView } from '$lib/stores/projectStore.js';
    import { invoke, convertFileSrc } from '@tauri-apps/api/core';
    import { get } from 'svelte/store';
    import EditGroupModal from '$lib/components/projectview/modals/EditGroupModal.svelte';

    // Props
    export let groupData; // Expected: { id, name, description, project_id }

    // Internal State
    let categorizedFiles = {
        audios: [],
        documents: [],
        images: [],
        tables: [],
        imported_transcripts: [],
        videos: [],
        others: [] // For any files that don't fit predefined categories
    };
    let isLoading = false;
    let errorMessage = null;
    let isEditGroupModalOpen = false;

    // Define category order and display names
    // Placeholder generic icons (SVGs can be inlined or imported as components if they exist)
    const GENERIC_ICONS = {
        audios: `<svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" fill="currentColor" class="bi bi-music-note-beamed" viewBox="0 0 16 16"><path d="M6 13c0 1.105-1.12 2-2.5 2S1 14.105 1 13s1.12-2 2.5-2 2.5.896 2.5 2m9-2c0 1.105-1.12 2-2.5 2s-2.5-.895-2.5-2 1.12-2 2.5-2 2.5.895 2.5 2"/><path fill-rule="evenodd" d="M14 11V2h1v9zM6 3v10H5V3z"/><path d="M5 2.905a1 1 0 0 1 .9-.995l8-.8a1 1 0 0 1 1.1.995V3L5 4z"/></svg>`,
        videos: `<svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" fill="currentColor" class="bi bi-film" viewBox="0 0 16 16"><path d="M0 1a1 1 0 0 1 1-1h14a1 1 0 0 1 1 1v14a1 1 0 0 1-1 1H1a1 1 0 0 1-1-1zm4 0v6h8V1zm8 8H4v6h8zM1 1v2h2V1zm2 3H1v2h2zM1 7v2h2V7zm2 3H1v2h2zm-2 3v2h2v-2zM15 1h-2v2h2zm-2 3v2h2V4zm2 3h-2v2h2zm-2 3v2h2v-2zm2 3h-2v2h2z"/></svg>`,
        documents: `<svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" fill="currentColor" class="bi bi-file-earmark-text" viewBox="0 0 16 16"><path d="M5.5 7a.5.5 0 0 0 0 1h5a.5.5 0 0 0 0-1zM5 9.5a.5.5 0 0 0 0 1h5a.5.5 0 0 0 0-1zM5 12a.5.5 0 0 0 0 1h2a.5.5 0 0 0 0-1z"/><path d="M9.5 0H4a2 2 0 0 0-2 2v12a2 2 0 0 0 2 2h8a2 2 0 0 0 2-2V4.5zm0 1v2A1.5 1.5 0 0 0 11 4.5h2V14a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1V2a1 1 0 0 1 1-1z"/></svg>`,
        images: `<svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" fill="currentColor" class="bi bi-image" viewBox="0 0 16 16"><path d="M6.002 5.5a1.5 1.5 0 1 1-3 0 1.5 1.5 0 0 1 3 0"/><path d="M2.002 1a2 2 0 0 0-2 2v10a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V3a2 2 0 0 0-2-2zm12 1a1 1 0 0 1 1 1v6.5l-3.777-1.947a.5.5 0 0 0-.577.093l-3.71 3.71-2.66-1.772a.5.5 0 0 0-.63.062L1.002 12V3a1 1 0 0 1 1-1z"/></svg>`,
        tables: `<svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" fill="currentColor" class="bi bi-table" viewBox="0 0 16 16"><path d="M0 2a2 2 0 0 1 2-2h12a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2H2a2 2 0 0 1-2-2zm15 2h-4v3h4zm0 4h-4v3h4zm0 4h-4v3h3a1 1 0 0 0 1-1zm-5 3v-3H6v3zm-5 0v-3H1v2a1 1 0 0 0 1 1zm-4-4h4V8H1zm0-4h4V4H1zm5-3v3h4V4zm4 4H6v3h4z"/></svg>`,
        imported_transcripts: `<svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" fill="currentColor" class="bi bi-chat-square-text" viewBox="0 0 16 16"><path d="M14 1a1 1 0 0 1 1 1v8a1 1 0 0 1-1 1h-2.5a2 2 0 0 0-1.6.8L8 14.333 6.1 11.8a2 2 0 0 0-1.6-.8H2a1 1 0 0 1-1-1V2a1 1 0 0 1 1-1zM2 0a2 2 0 0 0-2 2v8a2 2 0 0 0 2 2h2.5a1 1 0 0 1 .8.4l1.9 2.533a1 1 0 0 0 1.6 0l1.9-2.533a1 1 0 0 1 .8-.4H14a2 2 0 0 0 2-2V2a2 2 0 0 0-2-2z"/><path d="M3 3.5a.5.5 0 0 1 .5-.5h9a.5.5 0 0 1 0 1h-9a.5.5 0 0 1-.5-.5M3 6a.5.5 0 0 1 .5-.5h9a.5.5 0 0 1 0 1h-9A.5.5 0 0 1 3 6m0 2.5a.5.5 0 0 1 .5-.5h5a.5.5 0 0 1 0 1h-5a.5.5 0 0 1-.5-.5"/></svg>`,
        others: `<svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" fill="currentColor" class="bi bi-file-earmark" viewBox="0 0 16 16"><path d="M14 4.5V14a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V2a2 2 0 0 1 2-2h5.5zM13.5 4H9V.5A1.5 1.5 0 0 0 7.5 2v1A1.5 1.5 0 0 0 9 4.5h2z"/></svg>`,
    };

    const CATEGORY_ORDER = [
        { key: 'audios', name: 'Audios', icon: GENERIC_ICONS.audios },
        { key: 'documents', name: 'Documents', icon: GENERIC_ICONS.documents },
        { key: 'images', name: 'Images', icon: GENERIC_ICONS.images },
        { key: 'tables', name: 'Tables', icon: GENERIC_ICONS.tables },
        { key: 'imported_transcripts', name: 'Transcripts', icon: GENERIC_ICONS.imported_transcripts },
        { key: 'videos', name: 'Videos', icon: GENERIC_ICONS.videos },
        { key: 'others', name: 'Others', icon: GENERIC_ICONS.others }
    ];

    async function fetchGroupContents() {
        // Use get(project) to access store values if outside reactive context or component markup
        const currentProject = get(project);
        if (!groupData || !groupData.id || !currentProject || !currentProject.id || !currentProject.xmlPath) {
            errorMessage = "Group data or project context is missing.";
            console.error("fetchGroupContents precondition failed:", { groupData, currentProject });
            return;
        }
        isLoading = true;
        errorMessage = null;
        try {
            const files = await invoke('get_group_contents', {
                projectXmlPathStr: currentProject.xmlPath,
                groupId: groupData.id
            });

            const newCategorizedFiles = { audios: [], documents: [], images: [], tables: [], imported_transcripts: [], videos: [], others: [] };
            (files || []).forEach(file => { // Ensure files is an array
                switch (file.file_type) {
                    case 'audio': newCategorizedFiles.audios.push(file); break;
                    case 'video': newCategorizedFiles.videos.push(file); break; // Added video to switch
                    case 'document': newCategorizedFiles.documents.push(file); break;
                    case 'image': newCategorizedFiles.images.push(file); break;
                    case 'table': newCategorizedFiles.tables.push(file); break;
                    case 'imported_transcript': newCategorizedFiles.imported_transcripts.push(file); break;
                    default: newCategorizedFiles.others.push(file); break;
                }
            });
            categorizedFiles = newCategorizedFiles;
        } catch (err) {
            console.error("Error fetching group contents:", err);
            errorMessage = typeof err === 'string' ? err : "Failed to load group contents.";
        } finally {
            isLoading = false;
        }
    }

    function handleFileDoubleClick(file) {
        if (!file || !file.relative_path || !file.file_type) return;

        const filePathToOpen = file.full_path; // Use full_path from AssociatedFile

        if (file.file_type === 'document') {
            prepareDocumentView(filePathToOpen, 'documents');
        } else if (file.file_type === 'table') {
            prepareDocumentView(filePathToOpen, 'tables');
        } else if (file.file_type === 'image') {
            prepareDocumentView(filePathToOpen, 'images');
        } else if (file.file_type === 'imported_transcript') {
            prepareImportedTranscriptView(filePathToOpen);
        } else if (file.file_type === 'audio' || file.file_type === 'video' || file.file_type === 'media_other') {
            prepareMediaNoteView(filePathToOpen);
        } else {
            console.warn("Unknown file type for double click:", file.file_type);
        }
    }

    // Reactive watch on groupData and specific project properties
    // Using get(project) inside the reactive block might be redundant if $project is used,
    // but ensures access if the block's timing is tricky with store updates.
    // For simplicity and directness, direct $: subscription to $project.id and $project.xmlPath is cleaner.
    $: if (groupData && groupData.id && $project.id && $project.xmlPath) {
        fetchGroupContents();
    } else if (!groupData || !$project.id || !$project.xmlPath) { // Added condition to clear if context is lost
        categorizedFiles = { audios: [], documents: [], images: [], tables: [], imported_transcripts: [], videos: [], others: [] };
        isLoading = false;
        errorMessage = null;
    }

    function handleGroupDetailsUpdated(event) {
        const updatedGroup = event.detail;
        groupData = { ...groupData, ...updatedGroup }; // Update local prop
        isEditGroupModalOpen = false;

        // Update project store
        project.update(p => {
            if (p.selectedGroupData && p.selectedGroupData.id === updatedGroup.id) {
                return { ...p, selectedGroupData: { ...p.selectedGroupData, ...updatedGroup } };
            }
            return p;
        });
        // Potentially dispatch global event for NotesLeftPanel to refresh all groups if name changed
        // For now, this view and the central selectedGroupData are updated.
    }
</script>

<div class="p-4 h-full flex flex-col bg-white dark:bg-gray-800 rounded-md shadow">
    {#if groupData}
        <!-- Header -->
        <div class="mb-4 pb-2 border-b border-gray-300 dark:border-gray-600">
            <div class="flex items-center justify-between">
                <h2 class="text-xl font-semibold text-gray-800 dark:text-gray-100">{groupData.name}</h2>
                <button
                    on:click={() => isEditGroupModalOpen = true}
                    title="Edit group details"
                    class="p-1 text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                >
                    <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="bi bi-pencil-square w-4 h-4" viewBox="0 0 16 16">
                        <path d="M15.502 1.94a.5.5 0 0 1 0 .706L14.459 3.69l-2-2L13.502.646a.5.5 0 0 1 .707 0l1.293 1.293zm-1.75 2.456-2-2L4.939 9.21a.5.5 0 0 0-.121.196l-.805 2.414a.25.25 0 0 0 .316.316l2.414-.805a.5.5 0 0 0 .196-.12l6.813-6.814z"/>
                        <path fill-rule="evenodd" d="M1 13.5A1.5 1.5 0 0 0 2.5 15h11a1.5 1.5 0 0 0 1.5-1.5v-6a.5.5 0 0 0-1 0v6a.5.5 0 0 1-.5.5h-11a.5.5 0 0 1-.5-.5v-11a.5.5 0 0 1 .5-.5H9a.5.5 0 0 0 0-1H2.5A1.5 1.5 0 0 0 1 2.5z"/>
                    </svg>
                </button>
            </div>
            {#if groupData.description && groupData.description.trim() !== ''}
                <p class="text-sm text-gray-600 dark:text-gray-400 mt-1">{groupData.description}</p>
            {:else}
                <p class="text-sm text-gray-400 dark:text-gray-500 mt-1 italic h-5">No description provided.</p>
            {/if}
        </div>

        <!-- Body -->
        <div class="flex-grow overflow-y-auto">
            {#if isLoading}
                <p class="text-gray-500 dark:text-gray-400 text-center py-8">Loading group contents...</p>
            {:else if errorMessage}
                <p class="text-red-500 dark:text-red-400 text-center py-8">Error: {errorMessage}</p>
            {:else}
                {#each CATEGORY_ORDER as category}
                    {@const filesInCategory = categorizedFiles[category.key]}
                    <div class="mb-6">
                        <h3 class="text-lg font-medium text-gray-700 dark:text-gray-200 mb-2">{category.name}</h3>
                        {#if filesInCategory && filesInCategory.length > 0}
                            <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-4">
                                {#each filesInCategory as file (file.relative_path)}
                                    <div
                                        class="flex flex-col items-center p-3 border border-gray-200 dark:border-gray-700 rounded-lg hover:shadow-md dark:hover:bg-gray-700 cursor-pointer transition-shadow"
                                        on:dblclick={() => handleFileDoubleClick(file)}
                                        on:keydown={(e) => { if (e.key === 'Enter' || e.key === ' ') handleFileDoubleClick(file); }}
                                        role="button"
                                        tabindex="0"
                                        title={file.name}
                                    >
                                        <div class="w-20 h-20 mb-2 flex items-center justify-center text-gray-500 dark:text-gray-400">
                                            {#if file.file_type === 'image' && file.full_path}
                                                <img src={convertFileSrc(file.full_path)} alt={file.name} class="max-w-full max-h-full object-contain rounded"/>
                                            {:else}
                                                {@html GENERIC_ICONS[category.key] || GENERIC_ICONS['others']}
                                            {/if}
                                        </div>
                                        <p class="text-sm text-center text-gray-700 dark:text-gray-300 w-full h-10 overflow-hidden leading-tight">{file.name}</p>
                                    </div>
                                {/each}
                            </div>
                        {:else if !isLoading} <!-- Only show "No files" if not loading -->
                            <p class="text-sm text-gray-400 dark:text-gray-500 italic">No {category.name.toLowerCase()} in this group.</p>
                        {/if}
                    </div>
                {/each}

                {@const totalFiles = Object.values(categorizedFiles).reduce((sum, arr) => sum + arr.length, 0)}
                {#if totalFiles === 0 && !isLoading}
                     <p class="text-gray-500 dark:text-gray-400 text-center py-8">This group is empty.</p>
                {/if}
            {/if}
        </div>
    {:else}
            <p class="text-gray-500 dark:text-gray-400 text-center py-8">No group selected.</p>
    {/if}
</div>

<EditGroupModal
    bind:showModal={isEditGroupModalOpen}
    groupData={groupData}
    on:groupUpdated={handleGroupDetailsUpdated}
    on:close={() => isEditGroupModalOpen = false}
/>

<style>
    /* Ensure grid items don't overflow their container excessively if names are too long */
    .grid div > p {
        max-width: 100%; /* Or specific width like '8rem' or '120px' */
    }
</style>
