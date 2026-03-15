<script>
    import { createEventDispatcher, onMount } from 'svelte';
    import { message } from '@tauri-apps/plugin-dialog';
    import { languageOptions } from '$lib/constants/transcriptionOptions.js';
    import { getDownloadedModels, getSelectedTranscriptionEngine } from '$lib/services/configureActions.js';
    import { configStatus } from '$lib/stores/configStatusStore.js';
    import ManageModelsModal from './ManageModelsModal.svelte';
    import { 
        Modal, 
        Button, 
        Label, 
        Select, 
        Checkbox,
        Alert
    } from 'flowbite-svelte';
    import { Mic, Settings2, AlertTriangle, Info } from 'lucide-svelte';

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

<Modal
    bind:open={showModal}
    size="md"
    autoclose={false}
    outsideclose={true}
    on:close={closeModal}
    backdropClass="fixed inset-0 z-[10000] bg-black/60 backdrop-blur-sm"
    dialogClass="fixed top-0 start-0 end-0 h-modal md:h-full z-[10001] w-full p-4 flex items-center justify-center"
    class="w-full p-0 overflow-hidden flex flex-col"
    headerClass="px-6 py-5 border-b border-gray-200 dark:border-gray-800 flex justify-between items-center bg-gray-50/50 dark:bg-gray-800/50"
>
    <div slot="header" class="flex items-center space-x-3 w-full">
        <div class="p-2 bg-blue-100 dark:bg-blue-900/30 rounded-lg">
            <Mic size={20} class="text-blue-600 dark:text-blue-400" />
        </div>
        <h3 class="text-lg font-bold text-gray-900 dark:text-white">Live Transcription Settings</h3>
    </div>

    <div class="p-6 space-y-5">
        {#if !$configStatus.python_libraries_installed}
            <Alert color="red" class="items-start">
                <AlertTriangle slot="icon" class="w-5 h-5 shrink-0" />
                <div class="ml-2">
                    <p class="text-sm font-medium">Required libraries are missing.</p>
                    <p class="text-xs mt-1">Please install Python dependencies in the Configure screen.</p>
                    <Button
                        color="red"
                        size="xs"
                        class="mt-3"
                        on:click={() => showManageModelsModal = true}
                    >
                        Configure
                    </Button>
                </div>
            </Alert>
        {:else if models.length === 0}
            <Alert color="blue" class="items-start">
                <Info slot="icon" class="w-5 h-5 shrink-0" />
                <div class="ml-2">
                    <p class="text-sm font-medium">No transcription models available.</p>
                    <p class="text-xs mt-1">Please download a model in the Configure screen.</p>
                    <Button
                        color="blue"
                        size="xs"
                        class="mt-3"
                        on:click={() => showManageModelsModal = true}
                    >
                        Configure
                    </Button>
                </div>
            </Alert>
        {:else}
            <div class="space-y-2">
                <Label for="model-select">Model</Label>
                <Select
                    id="model-select"
                    items={models.map(m => ({ value: m.name, name: m.name }))}
                    bind:value={selectedModel}
                    placeholder="Select a model"
                />
            </div>
        {/if}

        <div class="space-y-2">
            <Label for="language-select">Language</Label>
            <Select
                id="language-select"
                items={languageOptions.map(l => ({ value: l.value, name: l.label }))}
                bind:value={selectedLanguage}
            />
        </div>

        <div class="space-y-3 pt-2">
            <Checkbox bind:checked={saveAudio}>Save transcription audio</Checkbox>
            <Checkbox bind:checked={addTimestamps}>Add Timestamps</Checkbox>
        </div>
    </div>

    <div slot="footer" class="flex justify-between items-center w-full">
        <button on:click={() => showManageModelsModal = true} class="text-sm text-blue-600 dark:text-blue-400 hover:underline flex items-center gap-1.5">
            <Settings2 size={14} />
            Manage Models
        </button>
        <div class="flex space-x-3">
            <Button color="alternative" on:click={closeModal}>Cancel</Button>
            <Button 
                color="blue" 
                on:click={handleConfirm} 
                disabled={models.length === 0 || !$configStatus.python_libraries_installed}
            >
                Start
            </Button>
        </div>
    </div>
</Modal>

<ManageModelsModal bind:showModal={showManageModelsModal} on:modelsChanged={loadModels} />
