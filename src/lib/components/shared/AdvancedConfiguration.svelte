<script>
    import { onMount } from 'svelte';
    import { invoke } from '@tauri-apps/api/core';
    import { Input, Label, Button, Select, Accordion, AccordionItem } from 'flowbite-svelte';
    import { CheckCircle, AlertTriangle, MessageSquareText, Users, Languages } from 'lucide-svelte';

    export let isBusy = false;

    let config = {
        diarization_device: 'auto',
        diarization_threads: 4,
        helsinki_batch_size: 8,
        nllb_batch_size: 1,
        num_threads: 4,
        device_preference: 'auto',
        quantization_preference: 'int8', // Default to int8 as per user request for performance option
        faster_whisper_compute_type: 'int8',
        faster_whisper_beam_size: 5,
        transcription_num_threads: 4, // Renamed for clarity in UI state
        transcription_device_preference: 'auto'
    };

    let platformInfo = null; // Initialize as null to indicate loading
    let statusMessage = '';
    let statusType = 'info'; // info, success, error
    

    const deviceOptions = [
        { value: 'auto', name: 'Auto (Recommended)' },
        { value: 'cpu', name: 'CPU' },
        { value: 'cuda', name: 'NVIDIA GPU (CUDA)' },
        { value: 'mps', name: 'Apple Silicon (MPS/Metal)' }
    ];

    const quantizationOptions = [
        { value: 'int8', name: 'Int8 (Fastest - Recommended)' },
        { value: 'float16', name: 'Float16 (Higher Precision)' }
    ];

    const computeTypeOptions = [
        { value: 'int8', name: 'Int8 (Fastest)' },
        { value: 'int8_float16', name: 'Int8 + Float16 (Hybrid)' },
        { value: 'float16', name: 'Float16 (Higher Precision)' }
    ];

    let originalQuantization = 'int8'; // To track changes

    $: recommendation = getHardwareRecommendation(platformInfo);
    $: helsinkiAuto = config.helsinki_batch_size === 0;
    $: nllbAuto = config.nllb_batch_size === 0;

    onMount(async () => {
        try {
            const savedTranslationConfig = await invoke('get_advanced_translation_config');
            if (savedTranslationConfig) {
                if (savedTranslationConfig.helsinki_batch_size !== undefined) config.helsinki_batch_size = savedTranslationConfig.helsinki_batch_size;
                if (savedTranslationConfig.nllb_batch_size !== undefined) config.nllb_batch_size = savedTranslationConfig.nllb_batch_size;
                if (savedTranslationConfig.num_threads !== undefined) config.num_threads = savedTranslationConfig.num_threads;
                if (savedTranslationConfig.device_preference !== undefined) config.device_preference = savedTranslationConfig.device_preference;
                if (savedTranslationConfig.diarization_device !== undefined) config.diarization_device = savedTranslationConfig.diarization_device;
                if (savedTranslationConfig.diarization_threads !== undefined) config.diarization_threads = savedTranslationConfig.diarization_threads;
                if (savedTranslationConfig.quantization_preference !== undefined && savedTranslationConfig.quantization_preference !== null) {
                     config.quantization_preference = savedTranslationConfig.quantization_preference;
                     originalQuantization = savedTranslationConfig.quantization_preference;
                }
            }

            const savedTranscriptionConfig = await invoke('get_advanced_transcription_config');
            if (savedTranscriptionConfig) {
                if (savedTranscriptionConfig.faster_whisper_compute_type !== undefined && savedTranscriptionConfig.faster_whisper_compute_type !== null) {
                    config.faster_whisper_compute_type = savedTranscriptionConfig.faster_whisper_compute_type;
                }
                if (savedTranscriptionConfig.faster_whisper_beam_size !== undefined) config.faster_whisper_beam_size = savedTranscriptionConfig.faster_whisper_beam_size;
                if (savedTranscriptionConfig.num_threads !== undefined) config.transcription_num_threads = savedTranscriptionConfig.num_threads;
                if (savedTranscriptionConfig.device_preference !== undefined && savedTranscriptionConfig.device_preference !== null) config.transcription_device_preference = savedTranscriptionConfig.device_preference;
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
            // Translation Config
            const translationPayload = {
                helsinki_batch_size: parseInt(config.helsinki_batch_size),
                nllb_batch_size: parseInt(config.nllb_batch_size),
                num_threads: parseInt(config.num_threads),
                device_preference: config.device_preference,
                diarization_device: config.diarization_device,
                diarization_threads: parseInt(config.diarization_threads),
                quantization_preference: config.quantization_preference
            };
            await invoke('set_advanced_translation_config', { newConfig: translationPayload });

            // Transcription Config
            const transcriptionPayload = {
                faster_whisper_compute_type: config.faster_whisper_compute_type,
                faster_whisper_beam_size: parseInt(config.faster_whisper_beam_size),
                num_threads: parseInt(config.transcription_num_threads),
                device_preference: config.transcription_device_preference
            };
            await invoke('set_advanced_transcription_config', { newConfig: transcriptionPayload });

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

    function resetTranscription() {
        config.faster_whisper_compute_type = 'int8';
        config.faster_whisper_beam_size = 5;
        config.transcription_num_threads = 4;
        config.transcription_device_preference = 'auto';
        statusMessage = 'Transcription settings reset (Click Save to apply).';
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

<div class="h-full overflow-y-auto">
    <div class="mb-6 p-3 bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-md">
        <p class="text-xs font-medium text-blue-700 dark:text-blue-400">{recommendation}</p>
    </div>
    
    <!-- Status Message Area (Moved up) -->
    {#if statusMessage && statusType !== 'info'}
        <div class="mb-4 flex items-center p-2 rounded-md" class:bg-green-100={statusType === 'success'} class:text-green-700={statusType === 'success'} class:bg-red-100={statusType === 'error'} class:text-red-700={statusType === 'error'} class:dark:bg-green-900={statusType === 'success'} class:dark:text-green-300={statusType === 'success'} class:dark:bg-red-900={statusType === 'error'} class:dark:text-red-300={statusType === 'error'}>
            {#if statusType === 'success'} <CheckCircle class="w-4 h-4 mr-2"/> {:else if statusType === 'error'} <AlertTriangle class="w-4 h-4 mr-2"/> {/if}
            <span class="text-sm">{statusMessage}</span>
        </div>
    {/if}

    <Accordion class="w-full space-y-4 bg-transparent border-0" flush multiple={false}>
        <!-- Transcription Panel -->
        <div class="bg-white dark:bg-gray-800/60 rounded-xl border border-gray-200 dark:border-gray-700 shadow-sm overflow-hidden">
            <AccordionItem open defaultClass="w-full flex items-center justify-between bg-gray-100 dark:bg-gray-800 px-4 py-3 border-b dark:border-gray-700 focus:outline-none hover:bg-gray-200 dark:hover:bg-gray-700 transition-colors">
                <span slot="header" class="flex items-center text-base font-medium text-gray-700 dark:text-gray-200">
                    <MessageSquareText size={18} class="mr-2 text-gray-500 dark:text-gray-400" />
                    Transcription Engine Parameters
                </span>

                <div class="p-4 space-y-4 bg-white dark:bg-gray-900">
                    <!-- General Settings -->
                    <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                        <div class="space-y-1">
                            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300">Device Preference</label>
                            <Select items={deviceOptions} bind:value={config.transcription_device_preference} />
                            <p class="text-[10px] text-gray-500">Auto will use GPU if available. 'CPU' bypasses GPU for both engines.</p>
                        </div>
                        <div class="space-y-1">
                            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300">CPU Threads</label>
                            <input type="number" bind:value={config.transcription_num_threads} min="1" max="32" class="input w-full" autocomplete="off" autocorrect="off" />
                            <p class="text-[10px] text-gray-500">Threads for inference (Faster-Whisper & Whisper.cpp).</p>
                        </div>
                    </div>

                    <div class="border-t border-gray-100 dark:border-gray-800 my-4"></div>

                    <h4 class="text-xs font-semibold text-gray-500 uppercase tracking-wider mb-3">Faster-Whisper Settings</h4>
                    <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                        <div class="space-y-1">
                            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300">Compute Type (Quantization)</label>
                            <Select items={computeTypeOptions} bind:value={config.faster_whisper_compute_type} />
                            <p class="text-[10px] text-gray-500">Precision of model weights. Int8 is fastest on CPU.</p>
                        </div>
                        <div class="space-y-1">
                            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300">Beam Size</label>
                            <input type="number" bind:value={config.faster_whisper_beam_size} min="1" max="10" class="input w-full" autocomplete="off" autocorrect="off" />
                            <p class="text-[10px] text-gray-500">Number of paths to search. 1 is greedy (fastest), 5 is standard.</p>
                        </div>
                    </div>

                    <!-- Panel Actions -->
                    <div class="pt-4 flex justify-end space-x-3 border-t border-gray-100 dark:border-gray-800 mt-4">
                        <button class="btn-secondary" on:click={resetTranscription} disabled={isBusy}>Reset Defaults</button>
                        <button class="btn-primary" on:click={handleSave} disabled={isBusy}>Save</button>
                    </div>
                </div>
            </AccordionItem>
        </div>

        <!-- Diarization Panel -->
        <div class="bg-white dark:bg-gray-800/60 rounded-xl border border-gray-200 dark:border-gray-700 shadow-sm overflow-hidden">
            <AccordionItem defaultClass="w-full flex items-center justify-between bg-gray-100 dark:bg-gray-800 px-4 py-3 border-b dark:border-gray-700 focus:outline-none hover:bg-gray-200 dark:hover:bg-gray-700 transition-colors">
                <span slot="header" class="flex items-center text-base font-medium text-gray-700 dark:text-gray-200">
                    <Users size={18} class="mr-2 text-gray-500 dark:text-gray-400" />
                    Diarization Engine Parameters
                </span>

                <div class="p-4 space-y-4 bg-white dark:bg-gray-900">
                    <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                        <!-- Device -->
                        <div class="space-y-1">
                            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300">Device Preference</label>
                            <Select items={deviceOptions} bind:value={config.diarization_device} />
                            <p class="text-[10px] text-gray-500">Force specific hardware. 'Auto' selects best available.</p>
                        </div>

                        <!-- Threads -->
                        <div class="space-y-1">
                            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300">CPU Threads</label>
                            <input type="number" bind:value={config.diarization_threads} min="1" max="32" class="input w-full" autocomplete="off" autocorrect="off" />
                            <p class="text-[10px] text-gray-500">Cores to use when running on CPU.</p>
                        </div>
                    </div>
                    <!-- Panel Actions -->
                    <div class="pt-4 flex justify-end space-x-3 border-t border-gray-100 dark:border-gray-800 mt-4">
                        <button class="btn-secondary" on:click={resetDiarization} disabled={isBusy}>Reset Defaults</button>
                        <button class="btn-primary" on:click={handleSave} disabled={isBusy}>Save</button>
                    </div>
                </div>
            </AccordionItem>
        </div>

        <!-- Translation Panel -->
        <div class="bg-white dark:bg-gray-800/60 rounded-xl border border-gray-200 dark:border-gray-700 shadow-sm overflow-hidden">
            <AccordionItem defaultClass="w-full flex items-center justify-between bg-gray-100 dark:bg-gray-800 px-4 py-3 border-b dark:border-gray-700 focus:outline-none hover:bg-gray-200 dark:hover:bg-gray-700 transition-colors">
                <span slot="header" class="flex items-center text-base font-medium text-gray-700 dark:text-gray-200">
                    <Languages size={18} class="mr-2 text-gray-500 dark:text-gray-400" />
                    Translation Engine Parameters
                </span>

                <div class="p-4 space-y-4 bg-white dark:bg-gray-900">
                    <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                        <!-- Device & Backend -->
                        <div class="space-y-1">
                            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300">Device Preference</label>
                            <Select items={deviceOptions} bind:value={config.device_preference} />
                            <p class="text-[10px] text-gray-500">Force specific hardware. 'Auto' selects best available.</p>
                        </div>
                        
                        <!-- Threads -->
                        <div class="space-y-1">
                            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300">CPU Threads</label>
                            <input type="number" bind:value={config.num_threads} min="1" max="32" class="input w-full" autocomplete="off" autocorrect="off" />
                            <p class="text-[10px] text-gray-500">Cores to use when running on CPU.</p>
                        </div>

                        <!-- Quantization -->
                        <div class="space-y-1 md:col-span-2 border-t pt-2 mt-2 dark:border-gray-800">
                            <label class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">Model Optimization (Quantization)</label>
                            <Select items={quantizationOptions} bind:value={config.quantization_preference} />
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
            </AccordionItem>
        </div>
    </Accordion>
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