<script>
    import { createEventDispatcher, onMount } from 'svelte';
    import { invoke } from '@tauri-apps/api/core';
    import { message } from '@tauri-apps/plugin-dialog';
    import { languageOptions } from '$lib/constants/transcriptionOptions.js';
    import ManageModelsModal from './ManageModelsModal.svelte';

    export let showModal = false;

    const dispatch = createEventDispatcher();

    let models = [];
    let selectedModel = '';
    let selectedLanguage = 'en';
    let saveAudio = false;
    let showManageModelsModal = false;

    async function loadModels() {
        try {
            models = await invoke('get_downloaded_models');
            if (models.length > 0) {
                selectedModel = models[0].name;
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
        dispatch('confirm', { model: selectedModel, language: selectedLanguage, saveAudio });
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
    <div class="bg-white dark:bg-gray-800 p-6 rounded-lg shadow-xl w-full max-w-md">
        <h2 class="text-lg font-semibold mb-4">Live Transcription Settings</h2>

        <div class="mb-4">
            <label for="model" class="block text-sm font-medium text-gray-700 dark:text-gray-300">Model</label>
            <select id="model" bind:value={selectedModel} class="mt-1 block w-full pl-3 pr-10 py-2 text-base border-gray-300 focus:outline-none focus:ring-indigo-500 focus:border-indigo-500 sm:text-sm rounded-md dark:bg-gray-700 dark:border-gray-600 dark:placeholder-gray-400 dark:text-white">
                {#each models as model}
                    <option value={model.name}>{model.name}</option>
                {/each}
            </select>
        </div>

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
                <input type="checkbox" bind:checked={saveAudio} class="rounded text-indigo-600 focus:ring-indigo-500 h-4 w-4 border-gray-300 dark:border-gray-600 dark:bg-gray-700">
                <span class="ml-2 text-sm text-gray-700 dark:text-gray-300">Save transcription audio</span>
            </label>
        </div>
        <div class="flex justify-between items-center mt-6">
            <button on:click={() => showManageModelsModal = true} class="text-sm text-blue-600 hover:underline">Manage Models</button>
            <div class="flex space-x-2">
                <button on:click={closeModal} class="px-4 py-2 bg-gray-200 text-gray-800 rounded-md hover:bg-gray-300">Cancel</button>
                <button on:click={handleConfirm} class="px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700">Start</button>
            </div>
        </div>
    </div>
</div>
{/if}

<ManageModelsModal bind:showModal={showManageModelsModal} on:modelsChanged={loadModels} />
