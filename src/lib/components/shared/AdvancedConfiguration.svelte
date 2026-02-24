<script>
    import { onMount } from 'svelte';
    import { invoke } from '@tauri-apps/api/core';
    import Dropdown from '$lib/components/shared/Dropdown.svelte';
    import { CheckCircle, AlertTriangle, ChevronDown, ChevronRight } from 'lucide-svelte';

    export let isBusy = false;

    let config = {
        diarization_device: 'auto',
        diarization_threads: 4,
        helsinki_batch_size: 8,
        nllb_batch_size: 1,
        num_threads: 4,
        device_preference: 'auto',
        quantization_preference: 'int8' // Default to int8 as per user request for performance option
    };

    let platformInfo = null; // Initialize as null to indicate loading
    let statusMessage = '';
    let statusType = 'info'; // info, success, error
    
    // Collapsible panel states
    let isDiarizationOpen = false;
    let isTranslationOpen = false;

    const deviceOptions = [
        { value: 'auto', label: 'Auto (Recommended)' },
        { value: 'cpu', label: 'CPU' },
        { value: 'cuda', label: 'NVIDIA GPU (CUDA)' },
        { value: 'mps', label: 'Apple Silicon (MPS/Metal)' }
    ];

    const quantizationOptions = [
        { value: 'int8', label: 'Int8 (Fastest - Recommended)' },
        { value: 'float16', label: 'Float16 (Higher Precision)' }
    ];

    let originalQuantization = 'int8'; // To track changes

    $: recommendation = getHardwareRecommendation(platformInfo);
    $: helsinkiAuto = config.helsinki_batch_size === 0;
    $: nllbAuto = config.nllb_batch_size === 0;

    onMount(async () => {
        try {
            const savedConfig = await invoke('get_advanced_translation_config');
            if (savedConfig) {
                if (savedConfig.helsinki_batch_size !== undefined) config.helsinki_batch_size = savedConfig.helsinki_batch_size;
                if (savedConfig.nllb_batch_size !== undefined) config.nllb_batch_size = savedConfig.nllb_batch_size;
                if (savedConfig.num_threads !== undefined) config.num_threads = savedConfig.num_threads;
                if (savedConfig.device_preference !== undefined) config.device_preference = savedConfig.device_preference;
                if (savedConfig.diarization_device !== undefined) config.diarization_device = savedConfig.diarization_device;
                if (savedConfig.diarization_threads !== undefined) config.diarization_threads = savedConfig.diarization_threads;
                if (savedConfig.quantization_preference !== undefined && savedConfig.quantization_preference !== null) {
                     config.quantization_preference = savedConfig.quantization_preference;
                     originalQuantization = savedConfig.quantization_preference;
                }
            }
            platformInfo = await invoke('get_platform_info');
            console.log('[AdvancedConfig] Platform Info:', platformInfo);
        } catch (e) {
            console.error("Failed to load advanced config:", e);
            statusMessage = `Error loading config: ${e}`;
            statusType = 'error';
            platformInfo = 'error';
        }
    });

    async function handleSave() {
        isBusy = true;
        statusMessage = '';
        try {
            // Ensure numbers are integers
            const payload = {
                helsinki_batch_size: parseInt(config.helsinki_batch_size),
                nllb_batch_size: parseInt(config.nllb_batch_size),
                num_threads: parseInt(config.num_threads),
                device_preference: config.device_preference,
                diarization_device: config.diarization_device,
                diarization_threads: parseInt(config.diarization_threads),
                quantization_preference: config.quantization_preference
            };
            await invoke('set_advanced_translation_config', { newConfig: payload });

            // Update original to prevent persistent warning
            originalQuantization = config.quantization_preference;

            statusMessage = 'Settings saved successfully.';
            statusType = 'success';
            setTimeout(() => statusMessage = '', 3000);
        } catch (e) {
            console.error("Failed to save advanced config:", e);
            statusMessage = `Error saving config: ${e}`;
            statusType = 'error';
        } finally {
            isBusy = false;
        }
    }

    function resetDiarization() {
        config.diarization_device = 'auto';
        config.diarization_threads = 4;
        statusMessage = 'Diarization settings reset (Click Save to apply).';
        statusType = 'info';
    }

    function resetTranslation() {
        config.helsinki_batch_size = 8;
        config.nllb_batch_size = 1;
        config.num_threads = 4;
        config.device_preference = 'auto';
        config.quantization_preference = 'int8';
        statusMessage = 'Translation settings reset (Click Save to apply).';
        statusType = 'info';
    }

    function toggleHelsinkiAuto(e) {
        if (e.target.checked) {
            config.helsinki_batch_size = 0;
        } else {
            config.helsinki_batch_size = 8;
        }
    }

    function toggleNllbAuto(e) {
        if (e.target.checked) {
            config.nllb_batch_size = 0;
        } else {
            config.nllb_batch_size = 1;
        }
    }

    function getHardwareRecommendation(info) {
        if (!info) return "Checking hardware...";
        if (info === 'error') return "Hardware detection failed.";

        const isMac = info.includes('macos') || info.includes('darwin') || info.includes('apple');
        const isARM = info.includes('aarch64') || info.includes('arm64');

        if (isMac) {
            if (isARM) {
                return "Detected: Apple Silicon (Native). Recommendation: Auto.";
            } else {
                return "Detected: macOS (Intel/Rosetta). Recommendation: Auto.";
            }
        } else if (info.includes('windows')) {
            return "Detected: Windows. Recommendation: Auto.";
        } else if (info.includes('linux')) {
            return "Detected: Linux. Recommendation: Auto.";
        } else {
            return `Detected: ${info}. Recommendation: Auto.`;
        }
    }
</script>

<div class="p-1 h-full overflow-y-auto">
    <div class="mb-4 p-3 bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-md">
        <p class="text-xs text-blue-700 dark:text-blue-400">{recommendation}</p>
    </div>
    
    <!-- Status Message Area (Moved up) -->
    {#if statusMessage}
        <div class="mb-4 flex items-center p-2 rounded-md" class:bg-green-100={statusType === 'success'} class:text-green-700={statusType === 'success'} class:bg-red-100={statusType === 'error'} class:text-red-700={statusType === 'error'} class:bg-gray-100={statusType === 'info'} class:text-gray-700={statusType === 'info'} class:dark:bg-green-900={statusType === 'success'} class:dark:text-green-300={statusType === 'success'} class:dark:bg-red-900={statusType === 'error'} class:dark:text-red-300={statusType === 'error'} class:dark:bg-gray-800={statusType === 'info'} class:dark:text-gray-300={statusType === 'info'}>
            {#if statusType === 'success'} <CheckCircle class="w-4 h-4 mr-2"/> {:else if statusType === 'error'} <AlertTriangle class="w-4 h-4 mr-2"/> {/if}
            <span class="text-sm">{statusMessage}</span>
        </div>
    {/if}

    <div class="space-y-6">
        <!-- Diarization Panel -->
        <div class="border dark:border-gray-700 rounded-md overflow-hidden">
            <button 
                class="w-full flex items-center justify-between bg-gray-100 dark:bg-gray-800 px-4 py-3 border-b dark:border-gray-700 focus:outline-none hover:bg-gray-200 dark:hover:bg-gray-700 transition-colors"
                on:click={() => isDiarizationOpen = !isDiarizationOpen}
            >
                <h3 class="font-medium text-gray-700 dark:text-gray-200">Diarization Engine Parameters</h3>
                {#if isDiarizationOpen}
                    <ChevronDown class="w-4 h-4 text-gray-500" />
                {:else}
                    <ChevronRight class="w-4 h-4 text-gray-500" />
                {/if}
            </button>
            
            {#if isDiarizationOpen}
                <div class="p-4 space-y-4 bg-white dark:bg-gray-900">
                    <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                        <!-- Device -->
                        <div class="space-y-1">
                            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300">Device Preference</label>
                            <Dropdown options={deviceOptions} bind:value={config.diarization_device} />
                            <p class="text-[10px] text-gray-500">Force specific hardware. 'Auto' selects best available (CUDA > MPS > CPU).</p>
                        </div>
                        
                        <!-- Threads -->
                        <div class="space-y-1">
                            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300">CPU Threads</label>
                            <input type="number" bind:value={config.diarization_threads} min="1" max="32" class="input w-full" />
                            <p class="text-[10px] text-gray-500">Cores to use when running on CPU.</p>
                        </div>
                    </div>
                    <!-- Panel Actions -->
                    <div class="pt-4 flex justify-end space-x-3 border-t border-gray-100 dark:border-gray-800 mt-4">
                        <button class="btn-secondary" on:click={resetDiarization} disabled={isBusy}>Reset Defaults</button>
                        <button class="btn-primary" on:click={handleSave} disabled={isBusy}>Save</button>
                    </div>
                </div>
            {/if}
        </div>

        <!-- Translation Panel -->
        <div class="border dark:border-gray-700 rounded-md overflow-hidden">
            <button 
                class="w-full flex items-center justify-between bg-gray-100 dark:bg-gray-800 px-4 py-3 border-b dark:border-gray-700 focus:outline-none hover:bg-gray-200 dark:hover:bg-gray-700 transition-colors"
                on:click={() => isTranslationOpen = !isTranslationOpen}
            >
                <h3 class="font-medium text-gray-700 dark:text-gray-200">Translation Engine Parameters</h3>
                {#if isTranslationOpen}
                    <ChevronDown class="w-4 h-4 text-gray-500" />
                {:else}
                    <ChevronRight class="w-4 h-4 text-gray-500" />
                {/if}
            </button>

            {#if isTranslationOpen}
                <div class="p-4 space-y-4 bg-white dark:bg-gray-900">
                    <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                        <!-- Device & Backend -->
                        <div class="space-y-1">
                            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300">Device Preference</label>
                            <Dropdown options={deviceOptions} bind:value={config.device_preference} />
                            <p class="text-[10px] text-gray-500">Force specific hardware. 'Auto' selects best available.</p>
                        </div>
                        
                        <!-- Threads -->
                        <div class="space-y-1">
                            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300">CPU Threads</label>
                            <input type="number" bind:value={config.num_threads} min="1" max="32" class="input w-full" />
                            <p class="text-[10px] text-gray-500">Cores to use when running on CPU.</p>
                        </div>

                        <!-- Quantization -->
                        <div class="space-y-1 md:col-span-2 border-t pt-2 mt-2 dark:border-gray-800">
                            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Model Optimization (Quantization)</label>
                            <Dropdown options={quantizationOptions} bind:value={config.quantization_preference} />
                            <p class="text-[10px] text-gray-500">
                                'Int8' is significantly faster on CPU. 'Float16' is higher precision.
                            </p>
                            {#if config.quantization_preference !== originalQuantization}
                                <div class="mt-2 p-2 bg-yellow-50 dark:bg-yellow-900/20 border border-yellow-200 dark:border-yellow-800 rounded text-xs text-yellow-800 dark:text-yellow-200 flex items-start">
                                    <AlertTriangle class="w-3 h-3 mr-1 mt-0.5 flex-shrink-0" />
                                    <span>Changing quantization requires re-downloading translation models to apply the new optimization format.</span>
                                </div>
                            {/if}
                        </div>

                        <!-- Batch Sizes -->
                        <div class="space-y-1">
                            <div class="flex justify-between items-center">
                                <label class="block text-sm font-medium text-gray-700 dark:text-gray-300">Batch Size for NLLB</label>
                                <label class="flex items-center space-x-1 cursor-pointer">
                                    <input type="checkbox" checked={nllbAuto} on:change={toggleNllbAuto} class="rounded border-gray-300 text-blue-600 focus:ring-blue-500 w-3 h-3" />
                                    <span class="text-xs text-gray-600 dark:text-gray-400">Auto</span>
                                </label>
                            </div>
                            {#if nllbAuto}
                                <input type="text" value="Dynamic (Hardware Optimized)" class="input w-full italic text-gray-500 bg-gray-100 dark:bg-gray-800" disabled />
                            {:else}
                                <input type="number" bind:value={config.nllb_batch_size} min="1" max="32" class="input w-full" />
                            {/if}
                            <p class="text-[10px] text-gray-500">Higher = Faster on GPU. Auto uses safe defaults.</p>
                        </div>
                        <div class="space-y-1">
                            <div class="flex justify-between items-center">
                                <label class="block text-sm font-medium text-gray-700 dark:text-gray-300">Batch Size for Helsinki</label>
                                <label class="flex items-center space-x-1 cursor-pointer">
                                    <input type="checkbox" checked={helsinkiAuto} on:change={toggleHelsinkiAuto} class="rounded border-gray-300 text-blue-600 focus:ring-blue-500 w-3 h-3" />
                                    <span class="text-xs text-gray-600 dark:text-gray-400">Auto</span>
                                </label>
                            </div>
                            {#if helsinkiAuto}
                                <input type="text" value="Dynamic (Hardware Optimized)" class="input w-full italic text-gray-500 bg-gray-100 dark:bg-gray-800" disabled />
                            {:else}
                                <input type="number" bind:value={config.helsinki_batch_size} min="1" max="64" class="input w-full" />
                            {/if}
                            <p class="text-[10px] text-gray-500">Small models can handle larger batches.</p>
                        </div>
                    </div>
                    <!-- Panel Actions -->
                    <div class="pt-4 flex justify-end space-x-3 border-t border-gray-100 dark:border-gray-800 mt-4">
                        <button class="btn-secondary" on:click={resetTranslation} disabled={isBusy}>Reset Defaults</button>
                        <button class="btn-primary" on:click={handleSave} disabled={isBusy}>Save</button>
                    </div>
                </div>
            {/if}
        </div>
    </div>
</div>

<style lang="postcss">
	.input {
		@apply bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-600 rounded-md px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500 text-gray-900 dark:text-gray-200;
	}
	.btn-primary {
		@apply px-4 py-2 bg-blue-600 text-white text-sm font-medium rounded-md hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 disabled:opacity-50 disabled:cursor-not-allowed;
	}
	.btn-secondary {
		@apply px-4 py-2 bg-white dark:bg-gray-700 text-gray-700 dark:text-gray-200 text-sm font-medium rounded-md border border-gray-300 dark:border-gray-600 hover:bg-gray-50 dark:hover:bg-gray-600 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 disabled:opacity-50 disabled:cursor-not-allowed;
	}
</style>