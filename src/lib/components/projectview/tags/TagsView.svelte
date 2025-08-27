<!-- src/lib/components/projectview/tags/TagsView.svelte -->
<script>
    import { onMount, afterUpdate } from 'svelte';
    import { invoke } from '@tauri-apps/api/core';
    import { confirm } from '@tauri-apps/plugin-dialog';
    import { TabulatorFull as Tabulator } from 'tabulator-tables';
    import { project } from '$lib/stores/projectStore.js';
    import { allTags, setTags } from '$lib/stores/tagStore.js';
    import { getAllTags } from '$lib/services/projectService.js';
    import SimpleTopBar from '../shared/SimpleTopBar.svelte';

    let selectedTag = null;
    let tagInfo = null;
    let isLoading = false;
    let description = '';
    let tableContainer;
    let tabulatorInstance = null;

    onMount(async () => {
        loadAllTags();
    });

    afterUpdate(() => {
        if (tagInfo && tableContainer && !tabulatorInstance) {
            initializeTable(tagInfo.highlights);
        } else if (tagInfo && tabulatorInstance) {
            tabulatorInstance.setData(tagInfo.highlights);
        } else if (!tagInfo && tabulatorInstance) {
            tabulatorInstance.destroy();
            tabulatorInstance = null;
        }
    });

    function initializeTable(data) {
        tabulatorInstance = new Tabulator(tableContainer, {
            data: data,
            layout: "fitData",
            columns: [
                { title: "Text", field: "text", widthGrow: 3, formatter: "textarea" },
                { title: "Source", field: "source", widthGrow: 1, formatter: (cell) => {
                    // In a future step, we can resolve this to the actual file name.
                    return "Placeholder Source";
                }},
                { title: "Type", field: "color", width: 80, formatter: (cell) => {
                    const color = cell.getValue();
                    return `<div style="display:flex; align-items:center;"><span style="display:inline-block; width:15px; height:15px; background-color:${color}; margin-right: 5px;"></span>Highlight</div>`;
                }},
            ],
            height: "100%",
            placeholder: "No highlights for this tag.",
        });
    }

    async function loadAllTags() {
        if (!$project.id || !$project.baseDirectory) return;
        try {
            isLoading = true;
            const tags = await getAllTags($project.id, $project.baseDirectory);
            setTags(tags);
        } catch (error) {
            console.error('Failed to load tags:', error);
        } finally {
            isLoading = false;
        }
    }

    async function handleSelectTag(tagName) {
        selectedTag = tagName;
        tagInfo = null;
        description = '';
        try {
            isLoading = true;
            tagInfo = await invoke('get_tag_info', {
                projectRootPathStr: $project.baseDirectory,
                projectId: $project.id,
                tagName: tagName,
            });
            if (tagInfo) {
                description = tagInfo.description;
            }
        } catch (error) {
            console.error(`Failed to load tag info for ${tagName}:`, error);
        } finally {
            isLoading = false;
        }
    }

    async function handleSaveDescription() {
        if (!selectedTag) return;
        try {
            await invoke('update_tag_info', {
                projectRootPathStr: $project.baseDirectory,
                tagName: selectedTag,
                newDescription: description,
            });
            // Optionally, show a success notification
        } catch (error) {
            console.error(`Failed to save description for ${selectedTag}:`, error);
            // Optionally, show an error notification
        }
    }

    async function handleDeleteTag() {
        if (!selectedTag) return;

        const confirmed = await confirm(`Are you sure you want to delete the tag "${selectedTag}"? This will remove the tag from all associated highlights and cannot be undone.`, {
            title: 'Confirm Deletion',
            type: 'warning',
        });

        if (confirmed) {
            try {
                await invoke('delete_tag', {
                    projectRootPathStr: $project.baseDirectory,
                    projectId: $project.id,
                    tagName: selectedTag,
                });
                selectedTag = null;
                tagInfo = null;
                description = '';
                await loadAllTags();
            } catch (error) {
                console.error(`Failed to delete tag ${selectedTag}:`, error);
                // Optionally, show an error notification
            }
        }
    }
</script>

<div class="flex flex-col h-full w-full bg-gray-100 dark:bg-app-bg-dark overflow-hidden">
    <SimpleTopBar />
    <div class="flex h-full w-full p-1 gap-1">
        <!-- Left Panel: List of all tags -->
        <div class="w-1/5 h-full bg-gray-50 dark:bg-gray-700 p-4 border-r border-gray-200 dark:border-gray-600">
        <h2 class="text-lg font-semibold mb-4">All Tags</h2>
        {#if $allTags.length > 0}
            <ul>
                {#each $allTags as tag}
                    <li
                        class="p-2 rounded-md cursor-pointer hover:bg-gray-200 dark:hover:bg-gray-600"
                        class:bg-blue-200={selectedTag === tag}
                        class:dark:bg-blue-800={selectedTag === tag}
                        on:click={() => handleSelectTag(tag)}
                    >
                        {tag}
                    </li>
                {/each}
            </ul>
        {:else}
            <p>No tags found in this project.</p>
        {/if}
    </div>

    <!-- Middle Panel: Tag details and highlights -->
    <div class="w-2/5 h-full p-4 overflow-y-auto">
        {#if selectedTag}
            {#if isLoading}
                <p>Loading tag information...</p>
            {:else if tagInfo}
                <h2 class="text-xl font-bold mb-2">{tagInfo.name}</h2>
                <div class="mb-4">
                    <label for="tag-description" class="block text-sm font-medium text-gray-700 dark:text-gray-300">Description</label>
                    <textarea id="tag-description" rows="3" class="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-indigo-500 focus:ring-indigo-500 sm:text-sm dark:bg-gray-800 dark:border-gray-600" bind:value={description}>
                    </textarea>
                </div>
                <div class="flex space-x-2 mb-4">
                    <button class="px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700" on:click={handleSaveDescription}>Save</button>
                    <button class="px-4 py-2 bg-red-600 text-white rounded-md hover:bg-red-700" on:click={handleDeleteTag}>Delete</button>
                </div>

                <h3 class="text-lg font-semibold mb-2">Highlights ({tagInfo.highlight_count})</h3>
                <div class="h-96" bind:this={tableContainer}></div>
            {/if}
        {:else}
            <div class="flex items-center justify-center h-full">
                <p class="text-gray-500">Select a tag to view its details.</p>
            </div>
        {/if}
    </div>

    <!-- Right Panel: Highlight content -->
    <div class="w-2/5 h-full bg-gray-50 dark:bg-gray-700 p-4 border-l border-gray-200 dark:border-gray-600">
        <h2 class="text-lg font-semibold mb-4">Highlight Content</h2>
        <p>Select a highlight to see its full content here.</p>
    </div>
    </div>
</div>
