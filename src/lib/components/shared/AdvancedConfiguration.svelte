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
        device_preference: 'auto'
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

    $: recommendation = getHardwareRecommendation(platformInfo);

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
                diarization_threads: parseInt(config.diarization_threads)
            };
            await invoke('set_advanced_translation_config', { newConfig: payload });
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

    async function handleReset() {
        config = {
            diarization_device: 'auto',
            diarization_threads: 4,
            helsinki_batch_size: 8,
            nllb_batch_size: 1,
            num_threads: 4,
            device_preference: 'auto'
        };
        statusMessage = 'Settings reset to defaults (Click Save to apply).';
        statusType = 'info';
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

                        <!-- Batch Sizes -->
                        <div class="space-y-1">
                            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300">Batch Size for NLLB</label>
                            <input type="number" bind:value={config.nllb_batch_size} min="1" max="32" class="input w-full" />
                            <p class="text-[10px] text-gray-500">Higher = Faster on GPU/MPS. Lower (1-4) safer for CPU.</p>
                        </div>
                        <div class="space-y-1">
                            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300">Batch Size for Helsinki</label>
                            <input type="number" bind:value={config.helsinki_batch_size} min="1" max="64" class="input w-full" />
                            <p class="text-[10px] text-gray-500">Small models can handle larger batches.</p>
                        </div>
                    </div>
                </div>
            {/if}
        </div>
    </div>

    <!-- Footer Actions -->
    <div class="mt-6 flex items-center justify-between pt-4 border-t border-gray-200 dark:border-gray-700">
        <div class="flex items-center">
            {#if statusMessage}
                {#if statusType === 'success'}
                    <CheckCircle class="w-4 h-4 text-green-500 mr-2" />
                    <span class="text-sm text-green-600">{statusMessage}</span>
                {:else if statusType === 'error'}
                    <AlertTriangle class="w-4 h-4 text-red-500 mr-2" />
                    <span class="text-sm text-red-600">{statusMessage}</span>
                {:else}
                    <span class="text-sm text-gray-600 dark:text-gray-400">{statusMessage}</span>
                {/if}
            {/if}
        </div>
        <div class="flex space-x-3">
            <button class="btn-secondary" on:click={handleReset} disabled={isBusy}>Reset to Defaults</button>
            <button class="btn-primary" on:click={handleSave} disabled={isBusy}>
                {isBusy ? 'Saving...' : 'Save'}
            </button>
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