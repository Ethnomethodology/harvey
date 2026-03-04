<script>
    import { onMount, onDestroy, createEventDispatcher } from 'svelte';
    import { invoke } from '@tauri-apps/api/core';
    import { listen } from '@tauri-apps/api/event';
    import { ask } from '@tauri-apps/plugin-dialog';
    import { open as openExternal } from '@tauri-apps/plugin-shell';
    import { fly, fade } from 'svelte/transition';
    import { 
        Check, 
        ChevronRight, 
        ChevronLeft, 
        Download, 
        PackageOpen, 
        Library,
        Languages, 
        Mic, 
        ShieldCheck, 
        X,
        AlertTriangle,
        Loader2,
        Search,
        ExternalLink,
        Key,
        Lock,
        Info
    } from 'lucide-svelte';
    import { configStatus, updateConfigStatus } from '$lib/stores/configStatusStore.js';
    import { 
        availableWhisperCppModels, 
        availableFasterWhisperModels 
    } from '$lib/constants/models.js';
    import { 
        downloadModel, 
        downloadFasterWhisperModel,
        downloadTranslationModel,
        fetchAvailableModels,
        getDownloadLocation,
        setSelectedTranscriptionEngine,
        setSelectedTranslationEngine
    } from '$lib/services/configureActions';
    import { languageMap } from '$lib/constants/languageMap.js';

    const dispatch = createEventDispatcher();

    let { showModal = $bindable(false) } = $props();

    let currentStep = $state(1);
    let showMoreInfo = $state(false);
    let platform = $state(''); 
    let isCudaAvailable = $state(false);
    let downloadLocation = $state('');

    // Step 1: Selections
    let transcriptionEngines = $state({ whisperCpp: false, fasterWhisper: false });
    let translationEngines = $state({ helsinki: false, nllb: false });

    // Step 2: Core Installation
    let installLogs = $state([]);
    let isInstalling = $state(false);
    let installProgress = $state({ phase: 'idle', current: 0, total: 0, currentItem: '' });
    let unlistenInstallLog;
    let unlistenInstallFinished;
    let unlistenDownloadProgress;
    let unlistenTranscriptionDownloadLog;
    let unlistenTranslationDownloadLog;
    let unlistenDiarizationDownloadLog;
    let unlistenDiarizationFinished;
    let downloadProgressData = $state({});

    // Step 3 & 4: Model Selections
    let selectedWhisperCppModels = $state(['ggml-base']); 
    let selectedFasterWhisperModels = $state([]);
    
    // Step 5 & 6: Translation Model Selections
    let helsinkiModels = $state([]);
    let nllbModels = $state([]);
    let helsinkiSearchQuery = $state('');
    let nllbSearchQuery = $state('');
    let isFetchingHelsinki = $state(false);
    let isFetchingNLLB = $state(false);
    let allAvailableTranslationModels = $state([]);

    // Step 7: Diarization
    let hfToken = $state('');
    let isVerifyingToken = $state(false);
    let diarizationAccessGranted = $state(false);
    let isDownloadingDiarization = $state(false);
    let diarizationDownloaded = $state(false);
    let diarizationError = $state('');
    let diarizationLogs = $state([]);

    let selectedModelsSummary = $derived.by(() => {
        const models = [];
        let totalSizeMiB = 0;

        const parseSize = (sizeStr) => {
            if (!sizeStr) return 0;
            const match = sizeStr.match(/^([\d.]+)\s*(MiB|GiB|MB|GB)$/i);
            if (!match) return 0;
            const val = parseFloat(match[1]);
            const unit = match[2].toLowerCase();
            if (unit === 'gib' || unit === 'gb') return val * 1024;
            return val;
        };

        if (transcriptionEngines.whisperCpp) {
            selectedWhisperCppModels.forEach(name => {
                const m = availableWhisperCppModels.find(am => am.name === name);
                if (m) {
                    models.push({ name: m.name, size: m.size, type: 'whisper.cpp' });
                    totalSizeMiB += parseSize(m.size);
                }
            });
        }
        if (transcriptionEngines.fasterWhisper) {
            selectedFasterWhisperModels.forEach(name => {
                const m = availableFasterWhisperModels.find(am => am.name === name);
                if (m) {
                    models.push({ name: m.name.split('/').pop(), size: m.size, type: 'faster-whisper' });
                    totalSizeMiB += parseSize(m.size);
                }
            });
        }
        helsinkiModels.forEach(id => {
            const m = allAvailableTranslationModels.find(am => am.id === id);
            if (m) {
                models.push({ name: formatModelDisplayName(m.id), size: 'Variable', type: 'Helsinki-NLP' });
                // Helsinki sizes aren't explicitly in the object, usually small ~30-50MB
                totalSizeMiB += 45; 
            }
        });
        nllbModels.forEach(id => {
            const m = allAvailableTranslationModels.find(am => am.id === id);
            if (m) {
                models.push({ name: formatModelDisplayName(m.id), size: 'Variable', type: 'NLLB' });
                if (m.id.includes('600M')) totalSizeMiB += 1200;
                else if (m.id.includes('1.3B')) totalSizeMiB += 2600;
                else totalSizeMiB += 5000;
            }
        });

        const totalGB = (totalSizeMiB / 1024).toFixed(1);
        return { models, totalGB, count: models.length };
    });

    let isMac = $derived(platform.startsWith('macos'));
    let recommendWhisperCpp = $derived(isMac);
    let recommendFasterWhisper = $derived(!isMac);
    let recommendHelsinki = $derived(!isCudaAvailable);

    // Trigger actions when reaching specific steps
    $effect(() => {
        if ((currentStep === 5 || currentStep === 6) && allAvailableTranslationModels.length === 0) {
            loadTranslationModels();
        }
        if (currentStep === 8) {
            checkDiarizationStatus();
        }
    });

    async function checkDiarizationStatus() {
        try {
            diarizationDownloaded = await invoke('check_diarization_model_access');
            const hasToken = await invoke('check_hf_auth_status');
            if (hasToken) {
                diarizationAccessGranted = true;
            }
        } catch (e) {
            console.error('Error checking diarization status:', e);
        }
    }

    function openLink(url) {
        openExternal(url).catch((err) => console.error(`Failed to open link: ${err}`));
    }

    onMount(async () => {
        try {
            platform = await invoke('get_platform_info');
            isCudaAvailable = await invoke('is_cuda_available_command');
            downloadLocation = await getDownloadLocation();
            
            transcriptionEngines.whisperCpp = recommendWhisperCpp;
            transcriptionEngines.fasterWhisper = recommendFasterWhisper;
            translationEngines.helsinki = true;
            
            await updateConfigStatus(true);
        } catch (e) {
            console.error('Error initializing wizard:', e);
        }

        unlistenDownloadProgress = await listen('download-progress', (event) => {
            const { model_name, downloaded_bytes, total_bytes } = event.payload;
            downloadProgressData[model_name] = { downloaded_bytes, total_bytes };
        });

        unlistenTranscriptionDownloadLog = await listen('transcription-download-log', (event) => {
            const { model_name, log_line } = event.payload;
            if (installProgress.phase === 'models' && installProgress.currentItem === model_name) {
                installLogs.push({ id: installLogs.length, message: log_line });
            }
        });

        unlistenTranslationDownloadLog = await listen('translation-download-log', (event) => {
            const { model_name, log_line } = event.payload;
            if (installProgress.phase === 'models' && installProgress.currentItem === model_name) {
                installLogs.push({ id: installLogs.length, message: log_line });
            }
        });

        unlistenDiarizationDownloadLog = await listen('diarization-installation-log', (event) => {
            if (currentStep === 8) {
                diarizationLogs.push({ id: diarizationLogs.length, message: event.payload.message });
            }
        });

        unlistenDiarizationFinished = await listen('diarization-installation-finished', () => {
            isDownloadingDiarization = false;
            diarizationDownloaded = true;
        });
    });

    onDestroy(() => {
        if (unlistenInstallLog) unlistenInstallLog();
        if (unlistenInstallFinished) unlistenInstallFinished();
        if (unlistenDownloadProgress) unlistenDownloadProgress();
        if (unlistenTranscriptionDownloadLog) unlistenTranscriptionDownloadLog();
        if (unlistenTranslationDownloadLog) unlistenTranslationDownloadLog();
        if (unlistenDiarizationDownloadLog) unlistenDiarizationDownloadLog();
        if (unlistenDiarizationFinished) unlistenDiarizationFinished();
    });

    async function close() {
        if (isInstalling || isDownloadingDiarization) return;
        
        if (installProgress.phase !== 'complete' || currentStep < 8) {
            const confirmed = await ask(
                'Are you sure you want to exit the Setup Wizard? You can always complete the environment setup and download models manually from the "Configure" tab later.',
                { title: 'Exit Setup?', kind: 'warning', okLabel: 'Exit', cancelLabel: 'Stay' }
            );
            if (!confirmed) return;
        }
        
        showModal = false;
        dispatch('close');
    }

    async function nextStep() {
        diarizationError = '';
        if (currentStep === 1) {
            currentStep = 2;
        } else if (currentStep === 2) {
            installProgress.phase = 'idle';
            installProgress.current = 0;
            installProgress.total = 0;
            installProgress.currentItem = '';
            
            if (transcriptionEngines.whisperCpp) currentStep = 3;
            else if (transcriptionEngines.fasterWhisper) currentStep = 4;
            else if (translationEngines.helsinki) currentStep = 5;
            else if (translationEngines.nllb) currentStep = 6;
            else currentStep = 7;
        } else if (currentStep === 3) {
            if (transcriptionEngines.fasterWhisper) currentStep = 4;
            else if (translationEngines.helsinki) currentStep = 5;
            else if (translationEngines.nllb) currentStep = 6;
            else currentStep = 7;
        } else if (currentStep === 4) {
            if (translationEngines.helsinki) currentStep = 5;
            else if (translationEngines.nllb) currentStep = 6;
            else currentStep = 7;
        } else if (currentStep === 5) {
            if (translationEngines.nllb) currentStep = 6;
            else currentStep = 7;
        } else if (currentStep === 6) {
            currentStep = 7;
        } else if (currentStep === 7) {
            currentStep = 8;
        }
    }

    function prevStep() {
        diarizationError = '';
        if (currentStep === 2) currentStep = 1;
        else if (currentStep === 3) currentStep = 2;
        else if (currentStep === 4) {
            if (transcriptionEngines.whisperCpp) currentStep = 3;
            else currentStep = 2;
        } else if (currentStep === 5) {
            if (transcriptionEngines.fasterWhisper) currentStep = 4;
            else if (transcriptionEngines.whisperCpp) currentStep = 3;
            else currentStep = 2;
        } else if (currentStep === 6) {
            if (translationEngines.helsinki) currentStep = 5;
            else if (transcriptionEngines.fasterWhisper) currentStep = 4;
            else if (transcriptionEngines.whisperCpp) currentStep = 3;
            else currentStep = 2;
        } else if (currentStep === 7) {
            if (translationEngines.nllb) currentStep = 6;
            else if (translationEngines.helsinki) currentStep = 5;
            else if (transcriptionEngines.fasterWhisper) currentStep = 4;
            else if (transcriptionEngines.whisperCpp) currentStep = 3;
            else currentStep = 2;
        } else if (currentStep === 8) {
            currentStep = 7;
        }
    }

    async function loadTranslationModels() {
        if (allAvailableTranslationModels.length > 0) return;
        try {
            isFetchingHelsinki = true;
            isFetchingNLLB = true;
            const fetched = await fetchAvailableModels();
            
            if (!fetched || fetched.length === 0) {
                allAvailableTranslationModels = [
                    { id: 'Helsinki-NLP/opus-mt-en-de', family: 'helsinki', src: 'en', tgt: 'de' },
                    { id: 'Helsinki-NLP/opus-mt-en-fr', family: 'helsinki', src: 'en', tgt: 'fr' },
                    { id: 'Helsinki-NLP/opus-mt-en-es', family: 'helsinki', src: 'en', tgt: 'es' },
                    { id: 'facebook/nllb-200-distilled-600M', family: 'nllb' }
                ];
            } else {
                allAvailableTranslationModels = fetched;
            }
        } catch (e) {
            console.error('Failed to fetch translation models:', e);
            allAvailableTranslationModels = [
                { id: 'Helsinki-NLP/opus-mt-en-de', family: 'helsinki', src: 'en', tgt: 'de' },
                { id: 'facebook/nllb-200-distilled-600M', family: 'nllb' }
            ];
        } finally {
            isFetchingHelsinki = false;
            isFetchingNLLB = false;
        }
    }

    function formatModelDisplayName(modelName) {
        if (!modelName) return 'Unknown Model';
        const parts = modelName.split('/');
		const baseName = parts[parts.length - 1] || modelName;

		if (baseName.toLowerCase().includes('nllb')) {
            if (baseName.includes('600M')) return "NLLB-200 Distilled (Small)";
            if (baseName.includes('1.3B')) return "NLLB-200 Distilled (Medium)";
            if (baseName.includes('3.3B')) return "NLLB-200 (Large)";
            return baseName;
		}

        if (parts.length === 2 && baseName.startsWith('opus-mt-')) {
            const langParts = baseName.split('-');
            if (langParts.length >= 4) {
                const fromCode = langParts[langParts.length - 2];
                const toCode = langParts[langParts.length - 1];
                const fromLang = languageMap.get(fromCode) || fromCode.toUpperCase();
                const toLang = languageMap.get(toCode) || toCode.toUpperCase();
                return `${fromLang} to ${toLang}`;
            }
        }
        return modelName; 
    }

    let filteredHelsinki = $derived(
        allAvailableTranslationModels
            .filter(m => m.family === 'helsinki')
            .filter(m => {
                const query = helsinkiSearchQuery.toLowerCase().trim();
                if (!query) return true;
                const displayName = formatModelDisplayName(m.id).toLowerCase();
                return m.id.toLowerCase().includes(query) || displayName.includes(query);
            })
            .slice(0, 50)
    );

    let filteredNLLB = $derived(
        allAvailableTranslationModels
            .filter(m => m.family === 'nllb')
            .filter(m => {
                const query = nllbSearchQuery.toLowerCase().trim();
                if (!query) return true;
                const displayName = formatModelDisplayName(m.id).toLowerCase();
                return m.id.toLowerCase().includes(query) || displayName.includes(query);
            })
            .slice(0, 50)
    );

    async function startCoreInstallation() {
        isInstalling = true;
        installLogs = [];
        installProgress.phase = 'libraries';
        
        try {
            if (unlistenInstallLog) unlistenInstallLog();
            unlistenInstallLog = await listen('installation-log', (event) => {
                installLogs.push({ id: installLogs.length, message: event.payload.message });
            });

            installProgress.currentItem = 'Core Libraries';
            await invoke('install_python_libraries');

            if (transcriptionEngines.whisperCpp) {
                installProgress.currentItem = 'whisper.cpp dependencies';
                await invoke('install_whisper_cpp_dependencies_command');
            }
            if (transcriptionEngines.fasterWhisper) {
                installProgress.currentItem = 'faster-whisper dependencies';
                await invoke('install_faster_whisper_dependencies_command');
            }

            installProgress.phase = 'complete';
            isInstalling = false;
            await updateConfigStatus(true);
        } catch (e) {
            console.error('Installation failed:', e);
            installLogs.push({ id: installLogs.length, message: `Error: ${e.message || e}` });
            isInstalling = false;
        }
    }

    async function startModelDownloads() {
        isInstalling = true;
        installProgress.phase = 'models';
        installLogs = []; 
        installLogs.push({ id: installLogs.length, message: 'Starting model downloads...' });
        
        try {
            if (transcriptionEngines.whisperCpp || transcriptionEngines.fasterWhisper) {
                const engine = transcriptionEngines.whisperCpp ? 'whisper-cpp' : 'faster-whisper';
                await setSelectedTranscriptionEngine(engine);
            }
            if (translationEngines.helsinki || translationEngines.nllb) {
                const family = translationEngines.helsinki ? 'helsinki' : 'nllb';
                await setSelectedTranslationEngine(family);
            }

            const modelsToDownload = [];
            
            if (transcriptionEngines.whisperCpp) {
                selectedWhisperCppModels.forEach(name => {
                    const m = availableWhisperCppModels.find(am => am.name === name);
                    if (m) modelsToDownload.push({ ...m, type: 'whisper-cpp' });
                });
            }
            if (transcriptionEngines.fasterWhisper) {
                selectedFasterWhisperModels.forEach(name => {
                    const m = availableFasterWhisperModels.find(am => am.name === name);
                    if (m) modelsToDownload.push({ ...m, type: 'faster-whisper' });
                });
            }
            
            helsinkiModels.forEach(id => {
                const m = allAvailableTranslationModels.find(am => am.id === id);
                if (m) modelsToDownload.push({ ...m, name: m.id, type: 'translation' });
            });
            nllbModels.forEach(id => {
                const m = allAvailableTranslationModels.find(am => am.id === id);
                if (m) modelsToDownload.push({ ...m, name: m.id, type: 'translation' });
            });

            installProgress.total = modelsToDownload.length;
            installProgress.current = 0;
            
            if (modelsToDownload.length === 0) {
                installLogs.push({ id: installLogs.length, message: 'No models selected.' });
            }

            for (const model of modelsToDownload) {
                installProgress.current++;
                installProgress.currentItem = model.name;
                installLogs.push({ id: installLogs.length, message: `Downloading ${model.name} (${installProgress.current}/${installProgress.total})...` });

                try {
                    if (model.type === 'whisper-cpp') {
                        await downloadModel(model, downloadLocation);
                    } else if (model.type === 'faster-whisper') {
                        await downloadFasterWhisperModel(model, downloadLocation);
                    } else if (model.type === 'translation') {
                        const idParts = model.name.split('opus-mt-')[1]?.split('-') || ['en', 'de'];
                        await downloadTranslationModel(idParts[0], idParts[1], downloadLocation, model.name, model.family);
                    }
                    installLogs.push({ id: installLogs.length, message: `Finished ${model.name}.` });
                } catch (e) {
                    installLogs.push({ id: installLogs.length, message: `Error downloading ${model.name}: ${e.message || e}` });
                }
            }

            installProgress.phase = 'complete';
            isInstalling = false;
            await updateConfigStatus(true);
        } catch (e) {
            console.error('Fatal error in startModelDownloads:', e);
            isInstalling = false;
        }
    }

    async function verifyHfToken() {
        if (!hfToken) return;
        isVerifyingToken = true;
        diarizationError = '';
        diarizationAccessGranted = false;
        
        try {
            const hasAccess = await invoke('check_gated_model_access', { token: hfToken });
            
            if (hasAccess === true) {
                await invoke('save_hf_auth_token', { token: hfToken });
                diarizationAccessGranted = true;
            } else {
                diarizationError = 'Access to the diarization model was not granted. Please accept the license on Hugging Face.';
            }
        } catch (e) {
            console.error('Token verification failed:', e);
            diarizationError = typeof e === 'string' ? e : (e.message || 'An error occurred while verifying the token.');
            diarizationAccessGranted = false;
        } finally {
            isVerifyingToken = false;
        }
    }

    async function downloadDiarization() {
        isDownloadingDiarization = true;
        diarizationLogs = [{ id: 0, message: 'Starting diarization model download...' }];
        try {
            await invoke('download_diarization_model');
            diarizationDownloaded = true;
            await updateConfigStatus(true);
        } catch (e) {
            diarizationError = `Download failed: ${e.message || e}`;
        } finally {
            isDownloadingDiarization = false;
        }
    }
</script>

{#if showModal}
<div class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/60 backdrop-blur-sm" transition:fade={{ duration: 200 }}>
    <div class="bg-white dark:bg-gray-900 w-full max-w-2xl rounded-xl shadow-2xl overflow-hidden flex flex-col max-h-[90vh]" transition:fly={{ y: 20, duration: 300 }}>
        <!-- Header -->
        <div class="px-6 py-4 border-b border-gray-200 dark:border-gray-800 flex items-center justify-between bg-gray-50 dark:bg-gray-900/50">
            <div class="flex items-center space-x-3">
                <div class="p-2 bg-blue-100 dark:bg-blue-900/30 rounded-lg">
                    <PackageOpen class="w-5 h-5 text-blue-600 dark:text-blue-400" />
                </div>
                <div>
                    <h2 class="text-lg font-bold text-gray-900 dark:text-gray-100">Setup Wizard</h2>
                    <p class="text-xs text-gray-500 dark:text-gray-400">Step {currentStep} of 8</p>
                </div>
            </div>
            <button on:click={close} class="p-2 hover:bg-gray-200 dark:hover:bg-gray-800 rounded-full transition-colors">
                <X class="w-5 h-5 text-gray-500" />
            </button>
        </div>

        <div class="w-full h-1 bg-gray-100 dark:bg-gray-800">
            <div class="h-full bg-blue-600 transition-all duration-500" style="width: {(currentStep / 8) * 100}%"></div>
        </div>

        <div class="flex-grow overflow-y-auto p-8">
            {#if currentStep === 1}
                <div in:fade>
                    <h3 class="text-xl font-bold mb-2">Choose Components</h3>
                    <p class="text-gray-600 dark:text-gray-400 mb-8">Select the AI engines and tools you'd like to use. We'll set up the required libraries next.</p>
                    <div class="space-y-6">
                        <div class="flex items-start p-4 rounded-xl border-2 border-blue-200 dark:border-blue-900/50 bg-blue-50/30">
                            <div class="pt-1 mr-4"><Library class="w-5 h-5 text-blue-600" /></div>
                            <div class="flex-grow">
                                <h4 class="font-bold">Core Libraries</h4>
                                <div class="text-sm text-gray-600 dark:text-gray-400 mt-1">
                                    <p>Harvey uses a local environment to manage required AI libraries.</p>
                                    <button on:click={() => showMoreInfo = !showMoreInfo} class="text-[10px] font-bold text-blue-600 mt-2 flex items-center">
                                        {showMoreInfo ? 'Show less' : 'More info'}
                                        <ChevronRight class="w-3 h-3 ml-0.5 transition-transform {showMoreInfo ? 'rotate-90' : ''}" />
                                    </button>
                                    {#if showMoreInfo}
                                        <div class="mt-3 space-y-2 border-t pt-3" transition:fade>
                                            <div class="text-xs">
                                                <strong>
                                                    <button class="text-blue-600 hover:underline" on:click={() => openLink('https://www.python.org/')}>Python</button> & 
                                                    <button class="text-blue-600 hover:underline" on:click={() => openLink('https://mamba.readthedocs.io/en/latest/user_guide/micromamba.html')}>micromamba</button>:
                                                </strong> Core runtime for executing AI models locally.
                                            </div>
                                            <div class="text-xs">
                                                <strong>
                                                    <button class="text-blue-600 hover:underline" on:click={() => openLink('https://ffmpeg.org/')}>FFmpeg</button>:
                                                </strong> For processing audio and video files.
                                            </div>
                                            <div class="text-xs">
                                                <strong>
                                                    <button class="text-blue-600 hover:underline" on:click={() => openLink('https://pytorch.org/')}>PyTorch</button> & 
                                                    <button class="text-blue-600 hover:underline" on:click={() => openLink('https://huggingface.co/docs/transformers/index')}>Transformers</button>:
                                                </strong> AI engine for running translation and analysis models.
                                            </div>
                                            <div class="text-xs">
                                                <strong>
                                                    <button class="text-blue-600 hover:underline" on:click={() => openLink('https://github.com/pyannote/pyannote-audio')}>pyannote.audio</button>:
                                                </strong> Specifically for speaker identification (diarization).
                                            </div>
                                            <div class="text-xs">
                                                <strong>
                                                    <button class="text-blue-600 hover:underline" on:click={() => openLink('https://pandoc.org/')}>Pandoc</button>:
                                                </strong> For converting and importing documents (e.g., MS Word).
                                            </div>
                                        </div>
                                    {/if}
                                </div>
                            </div>
                        </div>
                        <div class="space-y-3">
                            <h4 class="text-sm font-bold text-gray-500 uppercase flex items-center"><Mic class="w-4 h-4 mr-2" /> Transcription Engines</h4>
                            <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                                <div 
                                    role="button"
                                    tabindex="0"
                                    on:click={() => transcriptionEngines.whisperCpp = !transcriptionEngines.whisperCpp} 
                                    on:keydown={(e) => e.key === 'Enter' && (transcriptionEngines.whisperCpp = !transcriptionEngines.whisperCpp)}
                                    class="flex flex-col p-4 rounded-xl border-2 text-left cursor-pointer transition-all {transcriptionEngines.whisperCpp ? 'border-blue-600 bg-blue-50/30' : 'border-gray-200 dark:border-gray-800 hover:border-gray-300 dark:hover:border-gray-700'}"
                                >
                                    <div class="flex justify-between items-start">
                                        <span class="font-bold">whisper.cpp</span>
                                        <button class="p-1 hover:bg-blue-100 dark:hover:bg-blue-900/50 rounded-md transition-colors text-blue-600" on:click|stopPropagation={() => openLink('https://github.com/ggerganov/whisper.cpp')}>
                                            <ExternalLink class="w-3.5 h-3.5" />
                                        </button>
                                    </div>
                                    <p class="text-xs text-gray-600">Lightweight, fast on Mac (Metal) and CPU.</p>
                                </div>
                                <div 
                                    role="button"
                                    tabindex="0"
                                    on:click={() => transcriptionEngines.fasterWhisper = !transcriptionEngines.fasterWhisper} 
                                    on:keydown={(e) => e.key === 'Enter' && (transcriptionEngines.fasterWhisper = !transcriptionEngines.fasterWhisper)}
                                    class="flex flex-col p-4 rounded-xl border-2 text-left cursor-pointer transition-all {transcriptionEngines.fasterWhisper ? 'border-blue-600 bg-blue-50/30' : 'border-gray-200 dark:border-gray-800 hover:border-gray-300 dark:hover:border-gray-700'}"
                                >
                                    <div class="flex justify-between items-start">
                                        <span class="font-bold">faster-whisper</span>
                                        <button class="p-1 hover:bg-blue-100 dark:hover:bg-blue-900/50 rounded-md transition-colors text-blue-600" on:click|stopPropagation={() => openLink('https://github.com/SYSTRAN/faster-whisper')}>
                                            <ExternalLink class="w-3.5 h-3.5" />
                                        </button>
                                    </div>
                                    <p class="text-xs text-gray-600">Blazing fast, optimized for NVIDIA GPUs.</p>
                                </div>
                            </div>
                        </div>
                        <div class="space-y-3">
                            <h4 class="text-sm font-bold text-gray-500 uppercase flex items-center"><Languages class="w-4 h-4 mr-2" /> Translation Engines</h4>
                            <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                                <div 
                                    role="button"
                                    tabindex="0"
                                    on:click={() => translationEngines.helsinki = !translationEngines.helsinki} 
                                    on:keydown={(e) => e.key === 'Enter' && (translationEngines.helsinki = !translationEngines.helsinki)}
                                    class="flex flex-col p-4 rounded-xl border-2 text-left cursor-pointer transition-all {translationEngines.helsinki ? 'border-blue-600 bg-blue-50/30' : 'border-gray-200 dark:border-gray-800 hover:border-gray-300 dark:hover:border-gray-700'}"
                                >
                                    <div class="flex justify-between items-start">
                                        <span class="font-bold">Helsinki-NLP</span>
                                        <button class="p-1 hover:bg-blue-100 dark:hover:bg-blue-900/50 rounded-md transition-colors text-blue-600" on:click|stopPropagation={() => openLink('https://huggingface.co/Helsinki-NLP')}>
                                            <ExternalLink class="w-3.5 h-3.5" />
                                        </button>
                                    </div>
                                    <p class="text-xs text-gray-600">Lightweight, optimized for language pairs.</p>
                                </div>
                                <div 
                                    role="button"
                                    tabindex="0"
                                    on:click={() => translationEngines.nllb = !translationEngines.nllb} 
                                    on:keydown={(e) => e.key === 'Enter' && (translationEngines.nllb = !translationEngines.nllb)}
                                    class="flex flex-col p-4 rounded-xl border-2 text-left cursor-pointer transition-all {translationEngines.nllb ? 'border-blue-600 bg-blue-50/30' : 'border-gray-200 dark:border-gray-800 hover:border-gray-300 dark:hover:border-gray-700'}"
                                >
                                    <div class="flex justify-between items-start">
                                        <span class="font-bold">NLLB (Meta)</span>
                                        <button class="p-1 hover:bg-blue-100 dark:hover:bg-blue-900/50 rounded-md transition-colors text-blue-600" on:click|stopPropagation={() => openLink('https://huggingface.co/facebook/nllb-200-distilled-600M')}>
                                            <ExternalLink class="w-3.5 h-3.5" />
                                        </button>
                                    </div>
                                    <p class="text-xs text-gray-600">Universal model supporting 200+ languages.</p>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
            {:else if currentStep === 2}
                <div in:fade>
                    <h3 class="text-xl font-bold mb-2">Environment Setup</h3>
                    {#if installProgress.phase === 'idle'}
                        <div class="mb-8 space-y-4">
                            <p class="text-gray-600 dark:text-gray-400 text-sm">
                                Harvey uses <strong>micromamba</strong> to install and manage required AI libraries locally on your device.
                            </p>
                            
                            <div class="p-4 rounded-xl bg-gray-50 dark:bg-gray-800/50 border border-gray-200 dark:border-gray-700">
                                <h4 class="text-xs font-bold text-gray-500 uppercase mb-3 flex items-center">
                                    <Check class="w-3 h-3 mr-1 text-green-500" /> Selected Components
                                </h4>
                                <div class="flex flex-wrap gap-2">
                                    <span class="px-3 py-1 bg-blue-100 dark:bg-blue-900/30 text-blue-700 dark:text-blue-300 rounded-full text-xs font-medium">Core Libraries</span>
                                    {#if transcriptionEngines.whisperCpp}
                                        <span class="px-3 py-1 bg-indigo-100 dark:bg-indigo-900/30 text-indigo-700 dark:text-indigo-300 rounded-full text-xs font-medium">whisper.cpp</span>
                                    {/if}
                                    {#if transcriptionEngines.fasterWhisper}
                                        <span class="px-3 py-1 bg-indigo-100 dark:bg-indigo-900/30 text-indigo-700 dark:text-indigo-300 rounded-full text-xs font-medium">faster-whisper</span>
                                    {/if}
                                    {#if translationEngines.helsinki}
                                        <span class="px-3 py-1 bg-teal-100 dark:bg-teal-900/30 text-teal-700 dark:text-teal-300 rounded-full text-xs font-medium">Helsinki-NLP</span>
                                    {/if}
                                    {#if translationEngines.nllb}
                                        <span class="px-3 py-1 bg-teal-100 dark:bg-teal-900/30 text-teal-700 dark:text-teal-300 rounded-full text-xs font-medium">NLLB-200</span>
                                    {/if}
                                </div>
                            </div>

                            <p class="text-[11px] text-gray-500 italic">
                                Once downloaded, everything runs offline on your device to ensure privacy. This process may take a few minutes.
                            </p>
                        </div>
                        
                        <button on:click={startCoreInstallation} class="px-8 py-3 bg-blue-600 text-white rounded-xl font-bold flex items-center space-x-2 mx-auto shadow-lg hover:bg-blue-700 transition-all">
                            <span>Start Installation</span> <ChevronRight class="w-5 h-5" />
                        </button>
                    {:else}
                        <div class="space-y-6">
                            <div class="p-4 rounded-xl bg-gray-50 dark:bg-gray-800 border">
                                <div class="flex justify-between items-center">
                                    <div>
                                        <h4 class="font-bold">{installProgress.phase === 'complete' ? 'Installation Ready!' : 'Installing Libraries...'}</h4>
                                        <p class="text-xs text-gray-500 font-mono mt-1">{installProgress.currentItem}</p>
                                    </div>
                                    {#if installProgress.phase !== 'complete'}<Loader2 class="w-5 h-5 animate-spin text-blue-600" />{:else}<Check class="w-6 h-6 text-green-500" />{/if}
                                </div>
                            </div>
                            <div class="bg-gray-900 rounded-lg p-4 font-mono text-[11px] text-gray-300 h-64 overflow-y-auto scrollbar-hide">
                                {#each installLogs as log}<div class="mb-1">{log.message}</div>{/each}
                                {#if isInstalling}<div class="animate-pulse">_</div>{/if}
                            </div>
                        </div>
                    {/if}
                </div>
            {:else if currentStep === 3}
                <div in:fade>
                    <h3 class="text-xl font-bold mb-2">Whisper.cpp Models</h3>
                    <p class="text-gray-600 dark:text-gray-400 text-sm mb-6">Select models to download from the transcription marketplace.</p>
                    <div class="grid grid-cols-1 gap-2">
                        {#each availableWhisperCppModels as model}
                            <button on:click={() => selectedWhisperCppModels = selectedWhisperCppModels.includes(model.name) ? selectedWhisperCppModels.filter(m => m !== model.name) : [...selectedWhisperCppModels, model.name]} class="flex items-center justify-between p-3 rounded-lg border {selectedWhisperCppModels.includes(model.name) ? 'border-blue-600 bg-blue-50' : ''}">
                                <div class="flex items-center space-x-3"><div class="w-5 h-5 border flex items-center justify-center">{#if selectedWhisperCppModels.includes(model.name)}<Check class="w-4 h-4" />{/if}</div><span class="font-bold text-sm">{model.name}</span></div>
                                <div class="text-xs font-mono text-gray-500">{model.size}</div>
                            </button>
                        {/each}
                    </div>
                </div>
            {:else if currentStep === 4}
                <div in:fade>
                    <h3 class="text-xl font-bold mb-2">Faster-Whisper Models</h3>
                    <p class="text-gray-600 dark:text-gray-400 text-sm mb-6">Select models to download from the transcription marketplace.</p>
                    <div class="grid grid-cols-1 gap-2 overflow-y-auto max-h-[400px]">
                        {#each availableFasterWhisperModels as model}
                            <button on:click={() => selectedFasterWhisperModels = selectedFasterWhisperModels.includes(model.name) ? selectedFasterWhisperModels.filter(m => m !== model.name) : [...selectedFasterWhisperModels, model.name]} class="flex items-center justify-between p-3 rounded-lg border {selectedFasterWhisperModels.includes(model.name) ? 'border-blue-600 bg-blue-50' : ''}">
                                <div class="flex items-center space-x-3"><div class="w-5 h-5 border flex items-center justify-center">{#if selectedFasterWhisperModels.includes(model.name)}<Check class="w-4 h-4" />{/if}</div><span class="font-bold text-sm">{model.name.split('/').pop()}</span></div>
                                <div class="text-xs font-mono text-gray-500">{model.size}</div>
                            </button>
                        {/each}
                    </div>
                </div>
            {:else if currentStep === 5}
                <div in:fade>
                    <h3 class="text-xl font-bold mb-2">Helsinki-NLP Models</h3>
                    <p class="text-[11px] text-gray-500 mb-4 italic">Lightweight, very fast on CPU, requires separate models for every language pair.</p>
                    <div class="relative mb-4">
                        <Search class="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-gray-400" />
                        <input 
                            type="text" 
                            bind:value={helsinkiSearchQuery} 
                            placeholder="Search language pairs..." 
                            class="w-full pl-10 pr-4 py-2 border rounded-lg bg-white dark:bg-gray-800 border-gray-200 dark:border-gray-700 focus:ring-2 focus:ring-blue-500 outline-none transition-all" 
                            autocomplete="off"
                            autocorrect="off"
                            autocapitalize="off"
                            spellcheck="false"
                        />
                    </div>
                    <div class="grid grid-cols-1 gap-2 overflow-y-auto max-h-[350px]">
                        {#if isFetchingHelsinki}<div class="py-8 text-center"><Loader2 class="w-6 h-6 animate-spin mx-auto mb-2" />Fetching...</div>
                        {:else}{#each filteredHelsinki as model}<button on:click={() => helsinkiModels = helsinkiModels.includes(model.id) ? helsinkiModels.filter(m => m !== model.id) : [...helsinkiModels, model.id]} class="flex items-center space-x-3 p-3 rounded-lg border {helsinkiModels.includes(model.id) ? 'border-blue-600 bg-blue-50 dark:bg-blue-900/20' : 'border-gray-200 dark:border-gray-800'}"><div class="w-5 h-5 border flex items-center justify-center">{#if helsinkiModels.includes(model.id)}<Check class="w-4 h-4" />{/if}</div><div class="text-left"><div class="font-bold text-sm">{formatModelDisplayName(model.id)}</div><div class="text-[10px] font-mono">{model.id}</div></div></button>{/each}{/if}
                    </div>
                </div>
            {:else if currentStep === 6}
                <div in:fade>
                    <h3 class="text-xl font-bold mb-2">NLLB Models</h3>
                    <p class="text-[11px] text-gray-500 mb-4 italic">Universal model supporting 200+ languages. Great for rare languages, but larger file size.</p>
                    <div class="relative mb-4">
                        <Search class="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-gray-400" />
                        <input 
                            type="text" 
                            bind:value={nllbSearchQuery} 
                            placeholder="Search models..." 
                            class="w-full pl-10 pr-4 py-2 border rounded-lg bg-white dark:bg-gray-800 border-gray-200 dark:border-gray-700 focus:ring-2 focus:ring-blue-500 outline-none transition-all" 
                            autocomplete="off"
                            autocorrect="off"
                            autocapitalize="off"
                            spellcheck="false"
                        />
                    </div>
                    <div class="grid grid-cols-1 gap-2 overflow-y-auto max-h-[350px]">
                        {#if isFetchingNLLB}<div class="py-8 text-center"><Loader2 class="w-6 h-6 animate-spin mx-auto mb-2" />Fetching...</div>
                        {:else}{#each filteredNLLB as model}<button on:click={() => nllbModels = nllbModels.includes(model.id) ? nllbModels.filter(m => m !== model.id) : [...nllbModels, model.id]} class="flex items-center space-x-3 p-3 rounded-lg border {nllbModels.includes(model.id) ? 'border-blue-600 bg-blue-50 dark:bg-blue-900/20' : 'border-gray-200 dark:border-gray-800'}"><div class="w-5 h-5 border flex items-center justify-center">{#if nllbModels.includes(model.id)}<Check class="w-4 h-4" />{/if}</div><div class="text-left"><div class="font-bold text-sm">{formatModelDisplayName(model.id)}</div><div class="text-[10px] font-mono">{model.id}</div></div></button>{/each}{/if}
                    </div>
                </div>
            {:else if currentStep === 7}
                <div in:fade>
                    <h3 class="text-xl font-bold mb-2">Download Models</h3>
                    {#if installProgress.phase !== 'models' && installProgress.phase !== 'complete'}
                        <p class="text-gray-600 dark:text-gray-400 text-sm mb-6">Review your selections. We'll download these models to your local device.</p>
                        
                        <div class="space-y-4 mb-8">
                            <div class="bg-gray-50 dark:bg-gray-800/50 rounded-xl border border-gray-200 dark:border-gray-700 overflow-hidden">
                                <div class="px-4 py-3 border-b border-gray-200 dark:border-gray-700 bg-gray-100/50 dark:bg-gray-800 flex justify-between items-center">
                                    <span class="text-xs font-bold uppercase text-gray-500 tracking-wider">Selected Models ({selectedModelsSummary.count})</span>
                                    <span class="text-xs font-bold text-blue-600 dark:text-blue-400">~{selectedModelsSummary.totalGB} GB Total</span>
                                </div>
                                <div class="max-h-48 overflow-y-auto p-2 space-y-1">
                                    {#if selectedModelsSummary.models.length === 0}
                                        <div class="p-4 text-center text-sm text-gray-500 italic text-balance">No models selected. You can add them later in the Configure tab.</div>
                                    {:else}
                                        {#each selectedModelsSummary.models as model}
                                            <div class="flex items-center justify-between p-2 rounded-lg hover:bg-white dark:hover:bg-gray-800 transition-colors">
                                                <div class="flex flex-col">
                                                    <span class="text-sm font-bold text-gray-800 dark:text-gray-200 line-clamp-1">{model.name}</span>
                                                    <span class="text-[10px] text-gray-500 uppercase font-mono">{model.type}</span>
                                                </div>
                                                <span class="text-xs font-mono text-gray-500 bg-gray-100 dark:bg-gray-900 px-2 py-0.5 rounded border border-gray-200 dark:border-gray-700">{model.size}</span>
                                            </div>
                                        {/each}
                                    {/if}
                                </div>
                            </div>

                            {#if selectedModelsSummary.count > 0}
                                <p class="text-[11px] text-gray-500 italic text-center px-4">
                                    Total size is an estimate. Download times vary based on your connection speed.
                                </p>
                            {/if}
                        </div>

                        <div class="flex flex-col items-center">
                            <button on:click={startModelDownloads} class="px-10 py-3 bg-blue-600 text-white rounded-xl font-bold flex items-center space-x-2 shadow-lg hover:bg-blue-700 transition-all active:scale-95">
                                <span>{selectedModelsSummary.count === 0 ? 'Skip to Final Step' : 'Start Downloads'}</span> 
                                <ChevronRight class="w-5 h-5" />
                            </button>
                        </div>
                    {:else}
                        <div class="space-y-6">
                            <div class="p-4 rounded-xl bg-gray-50 dark:bg-gray-800 border border-gray-200 dark:border-gray-700">
                                <div class="flex justify-between items-center mb-4">
                                    <div><h4 class="font-bold">{installProgress.phase === 'complete' ? 'Finished!' : `Downloading (${installProgress.current}/${installProgress.total})`}</h4><p class="text-xs text-gray-500 font-mono mt-1">{installProgress.currentItem}</p></div>
                                    {#if installProgress.phase !== 'complete'}<Loader2 class="w-5 h-5 animate-spin text-blue-600" />{:else}<div class="w-6 h-6 rounded-full bg-green-500 flex items-center justify-center text-white"><Check class="w-4 h-4" /></div>{/if}
                                </div>
                                {#if installProgress.phase === 'models'}<div class="w-full h-2 bg-gray-200 dark:bg-gray-700 rounded-full overflow-hidden"><div class="h-full bg-blue-600 transition-all duration-300" style="width: {(installProgress.current / installProgress.total) * 100}%"></div></div>{/if}
                            </div>
                            {#if installProgress.phase === 'models' && downloadProgressData[installProgress.currentItem]}
                                {@const dp = downloadProgressData[installProgress.currentItem]}
                                <div class="px-2">
                                    <div class="flex justify-between text-[10px] text-gray-500 mb-1 font-mono uppercase"><span>{installProgress.currentItem}</span><span>{(dp.downloaded_bytes / 1048576).toFixed(1)} / {dp.total_bytes ? (dp.total_bytes / 1048576).toFixed(1) : '?'} MB</span></div>
                                    <div class="w-full h-1 bg-gray-100 dark:bg-gray-800 rounded-full overflow-hidden"><div class="h-full bg-indigo-500 transition-all duration-100" style="width: {dp.total_bytes ? (dp.downloaded_bytes / dp.total_bytes) * 100 : 0}%"></div></div>
                                </div>
                            {/if}
                            <div class="bg-gray-900 rounded-lg p-4 font-mono text-[11px] text-gray-300 h-48 overflow-y-auto scrollbar-hide border border-gray-800">
                                {#each installLogs as log}<div class="mb-1">{log.message}</div>{/each}
                            </div>
                        </div>
                    {/if}
                </div>
            {:else if currentStep === 8}
                <div in:fade={{ duration: 200 }}>
                    <div class="flex items-center space-x-3 mb-6"><div class="p-2 bg-amber-100 dark:bg-amber-900/30 rounded-lg"><ShieldCheck class="w-6 h-6 text-amber-600 dark:text-amber-400" /></div><h3 class="text-xl font-bold">Diarization Setup</h3></div>
                    
                    {#if diarizationDownloaded}
                        <div class="flex flex-col items-center justify-center py-12 space-y-4" in:fade>
                            <div class="w-16 h-16 bg-green-100 dark:bg-green-900/30 rounded-full flex items-center justify-center text-green-600 dark:text-green-400"><Check class="w-10 h-10 stroke-[3]" /></div>
                            <div class="text-center"><h4 class="text-lg font-bold text-gray-900 dark:text-gray-100">Model Installed</h4><p class="text-sm text-gray-500">Speaker identification is ready to use.</p></div>
                        </div>
                    {:else}
                        <p class="text-sm text-gray-600 dark:text-gray-400 mb-6">
                            Speaker diarization is the process of identifying and separating different speakers in an audio file.
                        </p>
                        <div class="space-y-6">
                            <div class="bg-gray-50 dark:bg-gray-800/50 rounded-xl p-4 border border-gray-200 dark:border-gray-700 space-y-4">
                                <h4 class="text-sm font-bold flex items-center"><Info class="w-4 h-4 mr-2 text-blue-500" /> Setup Instructions</h4>
                                <ol class="text-xs text-gray-600 dark:text-gray-400 space-y-3 list-decimal ml-4">
                                    <li><strong>Accept Model License:</strong> Harvey uses the gated <button class="text-blue-600 dark:text-blue-400 hover:underline font-medium" on:click={() => openLink('https://huggingface.co/pyannote/speaker-diarization-3.1')}>pyannote/speaker-diarization-3.1</button> model. Visit the link and click "Agree".</li>
                                    <li><strong>Create Access Token:</strong> Go to <button class="text-blue-600 dark:text-blue-400 hover:underline font-medium" on:click={() => openLink('https://huggingface.co/settings/tokens')}>HF Settings</button> and create a <strong>Read</strong> token.</li>
                                    <li><strong>Verify & Download:</strong> Paste your token below and click verify.</li>
                                </ol>
                            </div>

                            <div class="space-y-2">
                                <label class="text-xs font-bold uppercase text-gray-500 flex items-center"><Key class="w-3 h-3 mr-1" /> HF Token</label>
                                <div class="flex space-x-2">
                                    <input type="password" bind:value={hfToken} on:input={() => { diarizationAccessGranted = false; diarizationError = ''; }} placeholder="hf_..." class="flex-grow border rounded-lg px-4 py-2 bg-white dark:bg-gray-800 border-gray-200 dark:border-gray-700 focus:ring-2 focus:ring-blue-500 outline-none" />
                                    <button on:click={verifyHfToken} disabled={!hfToken || isVerifyingToken || isDownloadingDiarization} class="px-6 py-2 bg-gray-900 dark:bg-gray-100 text-white dark:text-gray-900 rounded-lg font-bold disabled:opacity-50 flex items-center">{#if isVerifyingToken}<Loader2 class="w-4 h-4 animate-spin mr-2" />{/if}Verify</button>
                                </div>
                                {#if diarizationError && !isDownloadingDiarization}<p class="text-[10px] text-red-600 dark:text-red-400 font-medium flex items-center"><AlertTriangle class="w-3 h-3 mr-1" /> {diarizationError}</p>{/if}
                            </div>
                            
                            {#if diarizationAccessGranted || isDownloadingDiarization}
                                <div class="space-y-4" in:fly={{ y: 10 }}>
                                    <div class="p-4 rounded-xl bg-blue-50 dark:bg-blue-900/10 border border-blue-100 dark:border-blue-900/30 flex items-center justify-between">
                                        <div class="flex items-center space-x-3"><ShieldCheck class="w-5 h-5 text-blue-600 dark:text-blue-400" /><span class="text-sm font-bold text-blue-700 dark:text-blue-300">Access Verified</span></div>
                                        {#if diarizationAccessGranted && !isDownloadingDiarization && !diarizationDownloaded}
                                            <button on:click={downloadDiarization} class="px-4 py-2 bg-blue-600 text-white text-xs font-bold rounded-lg hover:bg-blue-700 flex items-center space-x-2 shadow-lg transition-all"><Download class="w-3.5 h-3.5" /><span>Download Model</span></button>
                                        {/if}
                                    </div>
                                    {#if diarizationLogs.length > 0}
                                        <div class="bg-gray-900 rounded-lg p-4 font-mono text-[10px] text-gray-300 h-40 overflow-y-auto scrollbar-hide border border-gray-800">
                                            {#each diarizationLogs as log}<div class="mb-1 opacity-80"><span class="text-blue-500 mr-2">›</span>{log.message}</div>{/each}
                                            {#if isDownloadingDiarization}<div class="flex items-center text-blue-400 mt-1"><Loader2 class="w-3 h-3 animate-spin mr-2" /><span>Processing...</span></div>{/if}
                                        </div>
                                    {/if}
                                </div>
                            {/if}
                        </div>
                    {/if}
                </div>
            {/if}
        </div>

        <div class="px-6 py-4 border-t flex items-center justify-between bg-gray-50 dark:bg-gray-900/50">
            <button on:click={prevStep} disabled={currentStep === 1 || isInstalling || isDownloadingDiarization} class="px-4 py-2 text-sm font-bold text-gray-600 disabled:opacity-30 flex items-center transition-colors"><ChevronLeft class="w-4 h-4 mr-1" /> Back</button>
            <div class="flex space-x-3">
                {#if currentStep === 2}
                    <button on:click={nextStep} disabled={installProgress.phase !== 'complete'} class="px-6 py-2 bg-blue-600 text-white rounded-lg font-bold disabled:opacity-50 flex items-center">Next <ChevronRight class="w-4 h-4 ml-1" /></button>
                {:else if currentStep < 7}
                    <button on:click={nextStep} class="px-6 py-2 bg-blue-600 text-white rounded-lg font-bold flex items-center">Next <ChevronRight class="w-4 h-4 ml-1" /></button>
                {:else if currentStep === 7}
                    <button on:click={nextStep} disabled={installProgress.phase !== 'complete'} class="px-6 py-2 bg-blue-600 text-white rounded-lg font-bold disabled:opacity-50 flex items-center">Next <ChevronRight class="w-4 h-4 ml-1" /></button>
                {:else}
                    <button on:click={close} disabled={isDownloadingDiarization} class="px-8 py-2 bg-green-600 text-white rounded-lg font-bold disabled:opacity-50">Finish Setup</button>
                {/if}
            </div>
        </div>
    </div>
</div>
{/if}

<style>
    .scrollbar-hide::-webkit-scrollbar { display: none; }
    .scrollbar-hide { -ms-overflow-style: none; scrollbar-width: none; }
</style>
