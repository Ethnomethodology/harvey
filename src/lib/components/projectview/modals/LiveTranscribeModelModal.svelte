<script>
    import { createEventDispatcher, onMount } from 'svelte';
    import { message } from '@tauri-apps/plugin-dialog';
    import { languageOptions } from '$lib/constants/transcriptionOptions.js';
    import { getDownloadedModels, getSelectedTranscriptionEngine } from '$lib/services/configureActions.js';
    import { configStatus } from '$lib/stores/configStatusStore.js';
    import ManageModelsModal from './ManageModelsModal.svelte';

    export let showModal = false;

    const dispatch = createEventDispatcher();

    let models = [];
    let selectedModel = '';
    let selectedLanguage = 'en';
    let saveAudio = false;
    let addTimestamps = false;
    let showManageModelsModal = false;

    async function loadModels() {
        try {
            const allModels = await getDownloadedModels();
            const selectedEngine = await getSelectedTranscriptionEngine();
            const family = selectedEngine || 'whisper-cpp';

            models = allModels.filter(m => {
                if (family === 'faster-whisper') {
                    return m.family === 'faster-whisper';
                } else {
                    return m.family === 'whisper-cpp' || (!m.family && !m.name.includes('/'));
                }
            });

            if (models.length > 0) {
                // Keep selection if still valid, otherwise select first
                if (!selectedModel || !models.some(m => m.name === selectedModel)) {
                    selectedModel = models[0].name;
                }
            } else {
                selectedModel = '';
            }
        } catch (error) {
            message(`Error loading models: ${error}`, { title: 'Error', type: 'error' });
        }
    }

    function handleConfirm() {
        if (!selectedModel) {
            message('Please select a model.', { title: 'Error', type: 'error' });
            return;
        }
        const modelObj = models.find(m => m.name === selectedModel);
        dispatch('confirm', { 
            model: selectedModel, 
            language: selectedLanguage, 
            saveAudio,
            addTimestamps,
            family: modelObj?.family || 'whisper-cpp'
        });
        closeModal();
    }

    function closeModal() {
        dispatch('close');
    }

    onMount(() => {
        if (showModal) {
            loadModels();
        }
    });

    $: if (showModal) {
        loadModels();
    }
</script>

{#if showModal}
<div class="fixed inset-0 z-50 flex items-center justify-center bg-black bg-opacity-50">
    <div class="bg-white dark:bg-gray-900 p-6 rounded-lg shadow-xl w-full max-w-md">
        <h2 class="text-lg font-semibold mb-4">Live Transcription Settings</h2>

        {#if !$configStatus.python_libraries_installed}
            <div class="mb-6 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 p-4 rounded-md text-center space-y-2">
                <p class="text-red-800 dark:text-red-300 font-medium">Required libraries are missing.</p>
                <p class="text-xs text-red-600 dark:text-red-400">Please install Python dependencies in the Configure screen.</p>
                <div class="flex justify-center mt-2">
                    <button
                        on:click={() => showManageModelsModal = true}
                        class="px-3 py-1.5 bg-red-100 dark:bg-red-900 text-red-800 dark:text-red-100 rounded border border-red-300 dark:border-red-600 hover:bg-red-200 dark:hover:bg-red-800 text-xs font-semibold transition-colors"
                    >
                        Configure
                    </button>
                </div>
            </div>
        {:else if models.length === 0}
            <div class="mb-6 bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 p-4 rounded-md text-center space-y-2">
                <p class="text-blue-800 dark:text-blue-300 font-medium">No transcription models available.</p>
                <p class="text-xs text-blue-600 dark:text-blue-400">Please download a model in the Configure screen.</p>
                <div class="flex justify-center mt-2">
                    <button
                        on:click={() => showManageModelsModal = true}
                        class="px-3 py-1.5 bg-blue-100 dark:bg-blue-800 text-blue-800 dark:text-blue-100 rounded border border-blue-200 dark:border-blue-700 hover:bg-blue-200 dark:hover:bg-blue-700 text-xs font-semibold transition-colors"
                    >
                        Configure
                    </button>
                </div>
            </div>
        {:else}
            <div class="mb-4">
                <label for="model" class="block text-sm font-medium text-gray-700 dark:text-gray-300">Model</label>
                <select id="model" bind:value={selectedModel} class="mt-1 block w-full pl-3 pr-10 py-2 text-base border-gray-300 focus:outline-none focus:ring-indigo-500 focus:border-indigo-500 sm:text-sm rounded-md dark:bg-gray-700 dark:border-gray-600 dark:placeholder-gray-400 dark:text-white">
                    {#each models as model}
                        <option value={model.name}>{model.name}</option>
                    {/each}
                </select>
            </div>
        {/if}

        <div class="mb-4">
            <label for="language" class="block text-sm font-medium text-gray-700 dark:text-gray-300">Language</label>
            <select id="language" bind:value={selectedLanguage} class="mt-1 block w-full pl-3 pr-10 py-2 text-base border-gray-300 focus:outline-none focus:ring-indigo-500 focus:border-indigo-500 sm:text-sm rounded-md dark:bg-gray-700 dark:border-gray-600 dark:placeholder-gray-400 dark:text-white">
                {#each languageOptions as lang}
                    <option value={lang.value}>{lang.label}</option>
                {/each}
            </select>
        </div>

        <div class="mb-4">
            <label class="flex items-center">
                <input type="checkbox" bind:checked={saveAudio} class="rounded text-indigo-600 focus:ring-indigo-500 h-4 w-4 border-gray-300 dark:border-gray-600 dark:bg-gray-700" autocomplete="off" autocorrect="off">
                <span class="ml-2 text-sm text-gray-700 dark:text-gray-300">Save transcription audio</span>
            </label>
        </div>

        <div class="mb-4">
            <label class="flex items-center">
                <input type="checkbox" bind:checked={addTimestamps} class="rounded text-indigo-600 focus:ring-indigo-500 h-4 w-4 border-gray-300 dark:border-gray-600 dark:bg-gray-700" autocomplete="off" autocorrect="off">
                <span class="ml-2 text-sm text-gray-700 dark:text-gray-300">Add Timestamps</span>
            </label>
        </div>
        <div class="flex justify-between items-center mt-6">
            <button on:click={() => showManageModelsModal = true} class="text-sm text-blue-600 hover:underline">Manage Models</button>
            <div class="flex space-x-2">
                <button on:click={closeModal} class="px-4 py-2 bg-gray-200 text-gray-800 rounded-md hover:bg-gray-300">Cancel</button>
                <button on:click={handleConfirm} disabled={models.length === 0 || !$configStatus.python_libraries_installed} class="px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed">Start</button>
            </div>
        </div>
    </div>
</div>
{/if}

<ManageModelsModal bind:showModal={showManageModelsModal} on:modelsChanged={loadModels} />
