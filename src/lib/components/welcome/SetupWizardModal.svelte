<script>
    import { onMount, onDestroy, createEventDispatcher } from 'svelte';
    import { Modal } from 'flowbite-svelte';
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
        MessageSquareText, 
        Users, 
        X,
        AlertTriangle,
        Loader2,
        Search,
        ExternalLink,
        Key,
        Lock,
        Info
    } from '@lucide/svelte';
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
    let selectedFasterWhisperModels = $state(['Systran/faster-whisper-base']);
    
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
    let showModelDetails = $state(false);
    let isCleaningUp = $state(false);
    let isCheckingDiarization = $state(false);

    function resetWizard() {
        currentStep = 1;
        showMoreInfo = false;
        transcriptionEngines.whisperCpp = recommendWhisperCpp;
        transcriptionEngines.fasterWhisper = recommendFasterWhisper;
        translationEngines.helsinki = true;
        translationEngines.nllb = false;
        installLogs = [];
        isInstalling = false;
        installProgress = { phase: 'idle', current: 0, total: 0, currentItem: '' };
        downloadProgressData = {};
        selectedWhisperCppModels = ['ggml-base'];
        selectedFasterWhisperModels = ['Systran/faster-whisper-base'];
        helsinkiModels = [];
        nllbModels = [];
        helsinkiSearchQuery = '';
        nllbSearchQuery = '';
        hfToken = '';
        isVerifyingToken = false;
        diarizationAccessGranted = false;
        isDownloadingDiarization = false;
        diarizationDownloaded = false;
        diarizationError = '';
        diarizationLogs = [];
        showModelDetails = false;
        isCleaningUp = false;
        isCheckingDiarization = false;
    }

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
                const isBig = m.id.toLowerCase().includes('tc-big');
                const estSize = isBig ? '~580 MiB' : '~300 MiB';
                const sizeVal = isBig ? 580 : 300;
                
                models.push({ name: formatModelDisplayName(m.id), size: estSize, type: 'Helsinki-NLP' });
                totalSizeMiB += sizeVal; 
            }
        });
        nllbModels.forEach(id => {
            const m = allAvailableTranslationModels.find(am => am.id === id);
            if (m) {
                let estSize = '~2.5 GiB';
                if (m.id.includes('600M')) {
                    estSize = '~2.5 GiB';
                    totalSizeMiB += 2560;
                } else if (m.id.includes('1.3B')) {
                    estSize = '~5.5 GiB';
                    totalSizeMiB += 5632;
                } else if (m.id.includes('3.3B')) {
                    estSize = '~17.6 GiB';
                    totalSizeMiB += 18022;
                } else {
                    estSize = '~2.5 GiB';
                    totalSizeMiB += 2560;
                }
                models.push({ name: formatModelDisplayName(m.id), size: estSize, type: 'NLLB' });
            }
        });

        const totalGB = (totalSizeMiB / 1024).toFixed(1);
        return { models, totalGB, count: models.length };
    });

    let isMac = $derived((platform || '').startsWith('macos'));
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
        isCheckingDiarization = true;
        try {
            diarizationDownloaded = await invoke('check_diarization_model_access');
            const hasToken = await invoke('check_hf_auth_status');
            if (hasToken) {
                diarizationAccessGranted = true;
            }
        } catch (e) {
            console.error('Error checking diarization status:', e);
        } finally {
            // Small delay to prevent a flash if it returns too fast
            setTimeout(() => {
                isCheckingDiarization = false;
            }, 600);
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

        const unlistenTranscriptionComplete = await listen('transcription-download-complete', (event) => {
            const model_name = event.payload;
            delete downloadProgressData[model_name];
        });

        const unlistenTranslationComplete = await listen('translation-download-complete', (event) => {
            const model_name = event.payload;
            delete downloadProgressData[model_name];
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
        if (isCleaningUp) return;

        const isActivelyInstalling = isInstalling || isDownloadingDiarization || (installProgress.phase !== 'complete' && installProgress.phase !== 'idle' && installProgress.phase !== 'models');
        const needsConfirm = isActivelyInstalling || currentStep < 8;
        
        if (needsConfirm) {
            const confirmed = await ask(
                'Are you sure you want to exit the Setup Wizard?\n\n' +
                'You can always install libraries and download models ' +
                'manually from the "Configure" tab later.',
                { title: 'Exit Setup?', kind: 'warning', okLabel: 'Exit', cancelLabel: 'Stay' }
            );
            if (!confirmed) return;

            // If we are currently installing core libraries or diarization components,
            // we delete the environment to prevent corruption from a partial install.
            if (isDownloadingDiarization || (isInstalling && installProgress.phase === 'libraries')) {
                isCleaningUp = true;
                try {
                    await invoke('delete_virtual_env');
                    // Definitively stop all backend sidecar processes by reloading the main window
                    // This is the only way to ensure the Tauri sidecar child process is killed 
                    // if it doesn't respond to standard termination signals.
                    window.location.reload();
                    return; 
                } catch (e) {
                    console.error('Cleanup failed during wizard exit:', e);
                } finally {
                    isCleaningUp = false;
                }
            }
        }
        
        resetWizard();
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

        if (parts.length === 2 && baseName && baseName.startsWith('opus-mt-')) {
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

<Modal 
    bind:open={showModal} 
    size="xl" 
    outsideclose={false} 
    class="w-full overflow-hidden flex flex-col max-h-[90vh] z-50"
    headerClass="px-6 py-4 border-b border-gray-200 dark:border-gray-800 bg-white dark:bg-gray-900 rounded-t-lg flex justify-between items-center"
    bodyClass="p-0 flex-grow overflow-hidden flex flex-col bg-white dark:bg-gray-900"
    footerClass="px-6 py-4 bg-gray-50/80 dark:bg-gray-800/80 backdrop-blur-md border-t border-gray-200 dark:border-gray-800 rounded-b-lg flex justify-between items-center"
>
    <!-- Header Area -->
    <svelte:fragment slot="header">
        <div class="flex items-center space-x-3">
            <div class="p-2 bg-blue-100 dark:bg-blue-900/30 rounded-lg">
                <PackageOpen class="w-5 h-5 text-blue-600 dark:text-blue-400" />
            </div>
            <div>
                <h2 class="text-lg font-bold text-gray-900 dark:text-gray-100">Setup Wizard</h2>
                <p class="text-xs text-gray-500 dark:text-gray-400">Step {currentStep} of 8</p>
            </div>
        </div>
    </svelte:fragment>

    <!-- Main Content Area -->
    <div class="h-1 bg-gray-100 dark:bg-gray-800 w-full relative">
        <div class="h-full bg-blue-500 transition-all duration-500 ease-out" style="width: {(currentStep / 8) * 100}%"></div>
    </div>

    <div class="flex-grow overflow-y-auto p-8">
        {#if isCleaningUp}
            <div class="h-full flex flex-col items-center justify-center space-y-4" in:fade>
                <Loader2 class="w-12 h-12 animate-spin text-blue-600" />
                <div class="text-center">
                    <h3 class="text-lg font-bold text-gray-900 dark:text-gray-100">Cleaning up...</h3>
                    <p class="text-sm text-gray-500">Removing partial installation to prevent environment corruption.</p>
                </div>
            </div>
        {:else if currentStep === 1}
            <div in:fade>
                <h3 class="text-xl font-bold mb-2 text-gray-900 dark:text-gray-100">Choose Components</h3>
                <p class="text-gray-600 dark:text-gray-400 mb-8">Select the AI engines and tools you'd like to use. We'll set up the required libraries next.</p>
                <div class="space-y-6">
                    <div class="flex items-start p-4 rounded-xl border-2 border-blue-200 dark:border-blue-900/50 bg-blue-50/30">
                        <div class="pt-1 mr-4"><Library class="w-5 h-5 text-blue-600" /></div>
                        <div class="flex-grow">
                            <h4 class="font-bold text-gray-900 dark:text-gray-100">Core Libraries</h4>
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
                        <h4 class="text-sm font-bold text-gray-500 uppercase flex items-center"><MessageSquareText class="w-4 h-4 mr-2" /> Transcription Engines</h4>
                        <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                            <div 
                                role="button"
                                tabindex="0"
                                on:click={() => transcriptionEngines.whisperCpp = !transcriptionEngines.whisperCpp} 
                                on:keydown={(e) => e.key === 'Enter' && (transcriptionEngines.whisperCpp = !transcriptionEngines.whisperCpp)}
                                class="flex flex-col p-4 rounded-xl border-2 text-left cursor-pointer transition-all {transcriptionEngines.whisperCpp ? 'border-blue-600 bg-blue-50/30' : 'border-gray-200 dark:border-gray-800 hover:border-gray-300 dark:hover:border-gray-700'}"
                            >
                                <div class="flex justify-between items-start">
                                    <span class="font-bold text-gray-900 dark:text-gray-100">whisper.cpp</span>
                                    <button class="p-1 hover:bg-blue-100 dark:hover:bg-blue-900/50 rounded-md transition-colors text-blue-600" on:click|stopPropagation={() => openLink('https://github.com/ggerganov/whisper.cpp')}>
                                        <ExternalLink class="w-3.5 h-3.5" />
                                    </button>
                                </div>
                                <p class="text-xs text-gray-600">Lightweight, fast on Mac (Metal) and Windows (CPU).</p>                            </div>
                            <div 
                                role="button"
                                tabindex="0"
                                on:click={() => transcriptionEngines.fasterWhisper = !transcriptionEngines.fasterWhisper} 
                                on:keydown={(e) => e.key === 'Enter' && (transcriptionEngines.fasterWhisper = !transcriptionEngines.fasterWhisper)}
                                class="flex flex-col p-4 rounded-xl border-2 text-left cursor-pointer transition-all {transcriptionEngines.fasterWhisper ? 'border-blue-600 bg-blue-50/30' : 'border-gray-200 dark:border-gray-800 hover:border-gray-300 dark:hover:border-gray-700'}"
                            >
                                <div class="flex justify-between items-start">
                                    <span class="font-bold text-gray-900 dark:text-gray-100">faster-whisper</span>
                                    <button class="p-1 hover:bg-blue-100 dark:hover:bg-blue-900/50 rounded-md transition-colors text-blue-600" on:click|stopPropagation={() => openLink('https://github.com/SYSTRAN/faster-whisper')}>
                                        <ExternalLink class="w-3.5 h-3.5" />
                                    </button>
                                </div>
                                <p class="text-xs text-gray-600">Fast, optimized for NVIDIA GPUs.</p>                            </div>
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
                                    <span class="font-bold text-gray-900 dark:text-gray-100">Helsinki-NLP</span>
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
                                    <span class="font-bold text-gray-900 dark:text-gray-100">NLLB (Meta)</span>
                                    <button class="p-1 hover:bg-blue-100 dark:hover:bg-blue-900/50 rounded-md transition-colors text-blue-600" on:click|stopPropagation={() => openLink('https://huggingface.co/facebook/nllb-200-distilled-600M')}>
                                        <ExternalLink class="w-3.5 h-3.5" />
                                    </button>
                                </div>
                                <p class="text-xs text-gray-600">Heavy model supporting 200+ languages.</p>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        {:else if currentStep === 2}
            <div in:fade>
                <h3 class="text-xl font-bold mb-2 text-gray-900 dark:text-gray-100">Environment Setup</h3>
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
                        <div class="p-4 rounded-xl bg-gray-50 dark:bg-gray-800 border border-gray-200 dark:border-gray-700">
                            <div class="flex justify-between items-center">
                                <div>
                                    <h4 class="font-bold text-gray-900 dark:text-gray-100">{installProgress.phase === 'complete' ? 'Installation Ready!' : 'Installing Libraries...'}</h4>
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
                <h3 class="text-xl font-bold mb-2 text-gray-900 dark:text-gray-100">Whisper.cpp Models</h3>
                <p class="text-gray-600 dark:text-gray-400 text-sm mb-6">Select models to download from HuggingFace.</p>
                <div class="grid grid-cols-1 gap-2">
                    {#each availableWhisperCppModels as model}
                        <div class="flex items-center p-3 rounded-lg border transition-all {selectedWhisperCppModels.includes(model.name) ? 'border-blue-600 bg-blue-50 dark:bg-blue-900/20' : 'border-gray-200 dark:border-gray-800'}">
                            <button 
                                on:click={() => selectedWhisperCppModels = selectedWhisperCppModels.includes(model.name) ? selectedWhisperCppModels.filter(m => m !== model.name) : [...selectedWhisperCppModels, model.name]} 
                                class="flex-grow flex items-start space-x-3 text-left focus:outline-none"
                            >
                                <div class="w-5 h-5 border rounded mt-0.5 flex flex-shrink-0 items-center justify-center bg-white dark:bg-gray-800">
                                    {#if selectedWhisperCppModels.includes(model.name)}<Check class="w-4 h-4 text-blue-600" />{/if}
                                </div>
                                <div class="flex flex-col min-w-0 pr-2">
                                    <div class="flex items-center space-x-2">
                                        <span class="font-bold text-sm text-gray-900 dark:text-gray-100">{model.name}</span>
                                        <span class="text-[10px] font-mono text-gray-500 bg-gray-100 dark:bg-gray-900 px-1.5 py-0.5 rounded border border-gray-200 dark:border-gray-700">{model.size}</span>
                                        {#if model.name === 'ggml-base'}
                                            <span class="text-[9px] font-bold text-blue-600 bg-blue-100 dark:bg-blue-900/40 px-1.5 py-0.5 rounded-full uppercase tracking-wider">Starter</span>
                                        {/if}
                                    </div>
                                    <p class="text-[11px] text-gray-500 line-clamp-1 italic mt-0.5">{model.description}</p>
                                </div>
                            </button>
                            {#if model.info_url}
                                <button 
                                    class="p-1.5 text-gray-400 hover:text-blue-600 hover:bg-blue-100 dark:hover:bg-blue-900/50 rounded-md transition-colors"
                                    title="View on Hugging Face"
                                    on:click|stopPropagation={() => openLink(model.info_url)}
                                >
                                    <ExternalLink class="w-3.5 h-3.5" />
                                </button>
                            {/if}
                        </div>
                    {/each}
                </div>
            </div>
        {:else if currentStep === 4}
            <div in:fade>
                <h3 class="text-xl font-bold mb-2 text-gray-900 dark:text-gray-100">Faster-Whisper Models</h3>
                <p class="text-gray-600 dark:text-gray-400 text-sm mb-6">Select models to download from HuggingFace.</p>
                <div class="grid grid-cols-1 gap-2 overflow-y-auto max-h-[400px]">
                    {#each availableFasterWhisperModels as model}
                        <div class="flex items-center p-3 rounded-lg border transition-all {selectedFasterWhisperModels.includes(model.name) ? 'border-blue-600 bg-blue-50 dark:bg-blue-900/20' : 'border-gray-200 dark:border-gray-800'}">
                            <button 
                                on:click={() => selectedFasterWhisperModels = selectedFasterWhisperModels.includes(model.name) ? selectedFasterWhisperModels.filter(m => m !== model.name) : [...selectedFasterWhisperModels, model.name]} 
                                class="flex-grow flex items-start space-x-3 text-left focus:outline-none"
                            >
                                <div class="w-5 h-5 border rounded mt-0.5 flex flex-shrink-0 items-center justify-center bg-white dark:bg-gray-800">
                                    {#if selectedFasterWhisperModels.includes(model.name)}<Check class="w-4 h-4 text-blue-600" />{/if}
                                </div>
                                <div class="flex flex-col min-w-0 pr-2">
                                    <div class="flex items-center space-x-2">
                                        <span class="font-bold text-sm text-gray-900 dark:text-gray-100">{model.name.split('/').pop()}</span>
                                        <span class="text-[10px] font-mono text-gray-500 bg-gray-100 dark:bg-gray-900 px-1.5 py-0.5 rounded border border-gray-200 dark:border-gray-700">{model.size}</span>
                                        {#if model.name === 'Systran/faster-whisper-base'}
                                            <span class="text-[9px] font-bold text-blue-600 bg-blue-100 dark:bg-blue-900/40 px-1.5 py-0.5 rounded-full uppercase tracking-wider">Starter</span>
                                        {/if}
                                    </div>
                                    <p class="text-[11px] text-gray-500 line-clamp-1 italic mt-0.5">{model.description}</p>
                                </div>
                            </button>
                            {#if model.info_url}
                                <button 
                                    class="p-1.5 text-gray-400 hover:text-blue-600 hover:bg-blue-100 dark:hover:bg-blue-900/50 rounded-md transition-colors"
                                    title="View on Hugging Face"
                                    on:click|stopPropagation={() => openLink(model.info_url)}
                                >
                                    <ExternalLink class="w-3.5 h-3.5" />
                                </button>
                            {/if}
                        </div>
                    {/each}
                </div>
            </div>
        {:else if currentStep === 5}
            <div in:fade>
                <h3 class="text-xl font-bold mb-2 text-gray-900 dark:text-gray-100">Helsinki-NLP Models</h3>
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
                    {:else}
                        {#each filteredHelsinki as model}
                            <div class="flex items-center p-3 rounded-lg border transition-all {helsinkiModels.includes(model.id) ? 'border-blue-600 bg-blue-50 dark:bg-blue-900/20' : 'border-gray-200 dark:border-gray-800'}">
                                <button 
                                    on:click={() => helsinkiModels = helsinkiModels.includes(model.id) ? helsinkiModels.filter(m => m !== model.id) : [...helsinkiModels, model.id]} 
                                    class="flex-grow flex items-start space-x-3 text-left focus:outline-none"
                                >
                                    <div class="w-5 h-5 border rounded mt-0.5 flex flex-shrink-0 items-center justify-center bg-white dark:bg-gray-800">
                                        {#if helsinkiModels.includes(model.id)}<Check class="w-4 h-4 text-blue-600" />{/if}
                                    </div>
                                    <div class="flex flex-col min-w-0 pr-2">
                                        <div class="font-bold text-sm text-gray-900 dark:text-gray-100">{formatModelDisplayName(model.id)}</div>
                                        <div class="text-[10px] font-mono text-gray-500">{model.id}</div>
                                    </div>
                                </button>
                                <button 
                                    class="p-1.5 text-gray-400 hover:text-blue-600 hover:bg-blue-100 dark:hover:bg-blue-900/50 rounded-md transition-colors"
                                    title="View on Hugging Face"
                                    on:click|stopPropagation={() => openLink(`https://huggingface.co/${model.id}`)}
                                >
                                    <ExternalLink class="w-3.5 h-3.5" />
                                </button>
                            </div>
                        {/each}
                    {/if}
                </div>
            </div>
        {:else if currentStep === 6}
            <div in:fade>
                <h3 class="text-xl font-bold mb-2 text-gray-900 dark:text-gray-100">NLLB Models</h3>
                <p class="text-[11px] text-gray-500 mb-4 italic">One model supporting 200+ languages. Great for rare languages, but very heavy and requires significant disk space.</p>
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
                    {:else}
                        {#each filteredNLLB as model}
                            <div class="flex items-center p-3 rounded-lg border transition-all {nllbModels.includes(model.id) ? 'border-blue-600 bg-blue-50 dark:bg-blue-900/20' : 'border-gray-200 dark:border-gray-800'}">
                                <button 
                                    on:click={() => nllbModels = nllbModels.includes(model.id) ? nllbModels.filter(m => m !== model.id) : [...nllbModels, model.id]} 
                                    class="flex-grow flex items-start space-x-3 text-left focus:outline-none"
                                >
                                    <div class="w-5 h-5 border rounded mt-0.5 flex flex-shrink-0 items-center justify-center bg-white dark:bg-gray-800">
                                        {#if nllbModels.includes(model.id)}<Check class="w-4 h-4 text-blue-600" />{/if}
                                    </div>
                                    <div class="flex flex-col min-w-0 pr-2">
                                        <div class="font-bold text-sm text-gray-900 dark:text-gray-100">{formatModelDisplayName(model.id)}</div>
                                        <div class="text-[10px] font-mono text-gray-500">{model.id}</div>
                                    </div>
                                </button>
                                <button 
                                    class="p-1.5 text-gray-400 hover:text-blue-600 hover:bg-blue-100 dark:hover:bg-blue-900/50 rounded-md transition-colors"
                                    title="View on Hugging Face"
                                    on:click|stopPropagation={() => openLink(`https://huggingface.co/${model.id}`)}
                                >
                                    <ExternalLink class="w-3.5 h-3.5" />
                                </button>
                            </div>
                        {/each}
                    {/if}
                </div>
            </div>
        {:else if currentStep === 7}
            <div in:fade>
                <h3 class="text-xl font-bold mb-2 text-gray-900 dark:text-gray-100">Download Models</h3>
                {#if installProgress.phase !== 'models' && installProgress.phase !== 'complete'}
                    <p class="text-gray-600 dark:text-gray-400 text-sm mb-6">Review your selections. We'll download these models to your local device.</p>
                    
                    <div class="space-y-4 mb-8">
                        <div class="bg-gray-50 dark:bg-gray-800/50 rounded-xl border border-gray-200 dark:border-gray-700 overflow-hidden transition-all duration-300">
                            <div class="px-4 py-3 bg-gray-100/50 dark:bg-gray-800 flex justify-between items-center">
                                <div class="flex items-center space-x-2">
                                    <span class="text-sm font-bold text-gray-700 dark:text-gray-200">{selectedModelsSummary.count} Models to download</span>
                                    {#if selectedModelsSummary.count > 0}
                                        <span class="text-gray-400">&bull;</span>
                                        <span class="text-sm font-medium text-blue-600 dark:text-blue-400">~{selectedModelsSummary.totalGB} GB Total</span>
                                    {/if}
                                </div>
                                {#if selectedModelsSummary.count > 0}
                                    <button 
                                        on:click={() => showModelDetails = !showModelDetails} 
                                        class="text-xs font-bold text-blue-600 hover:text-blue-700 dark:text-blue-400 dark:hover:text-blue-300 hover:underline flex items-center space-x-1"
                                    >
                                        <span>{showModelDetails ? 'Hide details' : 'Show details'}</span>
                                    </button>
                                {/if}
                            </div>

                            {#if showModelDetails && selectedModelsSummary.count > 0}
                                <div class="p-2 space-y-1 border-t border-gray-200 dark:border-gray-700 bg-white/50 dark:bg-gray-900/30" in:fade={{ duration: 200 }}>
                                    {#each selectedModelsSummary.models as model}
                                        <div class="flex items-center justify-between p-2 rounded-lg hover:bg-white dark:hover:bg-gray-800 transition-colors">
                                            <div class="flex flex-col">
                                                <span class="text-sm font-bold text-gray-800 dark:text-gray-200 line-clamp-1">{model.name}</span>
                                                <span class="text-[10px] text-gray-500 uppercase font-mono">{model.type}</span>
                                            </div>
                                            <span class="text-xs font-mono text-gray-500 bg-gray-100 dark:bg-gray-900 px-2 py-0.5 rounded border border-gray-200 dark:border-gray-700">{model.size}</span>
                                        </div>
                                    {/each}
                                </div>
                            {/if}

                            {#if selectedModelsSummary.count === 0}
                                <div class="p-6 text-center text-sm text-gray-500 italic border-t border-gray-200 dark:border-gray-700">
                                    No models selected. You can add them later in the Configure tab.
                                </div>
                            {/if}
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
                                <div><h4 class="font-bold text-gray-900 dark:text-gray-100">{installProgress.phase === 'complete' ? 'Finished!' : `Downloading (${installProgress.current}/${installProgress.total})`}</h4><p class="text-xs text-gray-500 font-mono mt-1">{installProgress.currentItem}</p></div>
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
                <div class="flex items-center space-x-3 mb-6">
                    <div class="p-2 bg-amber-100 dark:bg-amber-900/30 rounded-lg">
                        <Users class="w-6 h-6 text-amber-600 dark:text-amber-400" />
                    </div>
                    <h3 class="text-xl font-bold text-gray-900 dark:text-gray-100">Diarization Setup</h3>
                </div>

                {#if isCheckingDiarization}
                    <div class="flex flex-col items-center justify-center py-12 space-y-4" in:fade>
                        <Loader2 class="w-10 h-10 animate-spin text-amber-500" />
                        <p class="text-sm font-medium text-gray-600 dark:text-gray-400">Verifying existing setup...</p>
                    </div>
                {:else}
                    <p class="text-sm text-gray-600 dark:text-gray-400 mb-8">
                        Speaker diarization automatically identifies and separates different speakers in an audio file. Harvey uses the gated <button on:click|preventDefault={() => openLink('https://huggingface.co/pyannote/speaker-diarization-3.1')} class="text-blue-600 dark:text-blue-400 hover:underline font-medium">pyannote/speaker-diarization-3.1</button> model. Follow the steps below to authenticate and download the model.
                    </p>

                    <ol class="relative text-gray-700 dark:text-gray-300 border-s border-gray-200 dark:border-gray-700 ml-3.5">
                        <!-- Step 1: Create Account -->
                        <li class="mb-10 ms-8">
                            <span class="absolute flex items-center justify-center w-8 h-8 rounded-full -start-4 ring-4 ring-white dark:ring-gray-900 {diarizationAccessGranted || diarizationDownloaded ? 'bg-green-100 text-green-600 dark:bg-green-900/30 dark:text-green-400' : 'bg-gray-100 text-gray-600 dark:bg-gray-800 dark:text-gray-400'}">
                                <span class="font-medium text-sm">1</span>
                            </span>
                            <h3 class="font-medium leading-tight text-gray-900 dark:text-white mb-2">Create HuggingFace Account</h3>
                            <p class="text-xs text-gray-600 dark:text-gray-400">If you don't have one, create a free HuggingFace account on their <button on:click|preventDefault={() => openLink('https://huggingface.co/join')} class="text-blue-600 hover:underline dark:text-blue-400 font-medium">website</button>.</p>
                        </li>

                        <!-- Step 2: Accept Agreement -->
                        <li class="mb-10 ms-8">
                            <span class="absolute flex items-center justify-center w-8 h-8 rounded-full -start-4 ring-4 ring-white dark:ring-gray-900 {diarizationAccessGranted || diarizationDownloaded ? 'bg-green-100 text-green-600 dark:bg-green-900/30 dark:text-green-400' : 'bg-gray-100 text-gray-600 dark:bg-gray-800 dark:text-gray-400'}">
                                <span class="font-medium text-sm">2</span>
                            </span>
                            <h3 class="font-medium leading-tight text-gray-900 dark:text-white mb-2">Accept Diarization Agreement</h3>
                            <p class="text-xs text-gray-600 dark:text-gray-400 mb-3">Accept the user agreement on the Pyannote HuggingFace page to unlock access to the model.</p>
                            <button on:click={() => openLink('https://huggingface.co/pyannote/speaker-diarization-3.1')} class="text-xs font-bold text-blue-600 hover:text-blue-700 dark:text-blue-400 flex items-center">
                                Open Pyannote Agreement <ExternalLink class="w-3 h-3 ml-1" />
                            </button>
                        </li>

                        <!-- Step 3: Generate Token -->
                        <li class="mb-10 ms-8">
                            <span class="absolute flex items-center justify-center w-8 h-8 rounded-full -start-4 ring-4 ring-white dark:ring-gray-900 {diarizationAccessGranted || diarizationDownloaded ? 'bg-green-100 text-green-600 dark:bg-green-900/30 dark:text-green-400' : 'bg-gray-100 text-gray-600 dark:bg-gray-800 dark:text-gray-400'}">
                                <span class="font-medium text-sm">3</span>
                            </span>
                            <h3 class="font-medium leading-tight text-gray-900 dark:text-white mb-2">Generate Access Token</h3>
                            <p class="text-xs text-gray-600 dark:text-gray-400">Generate an access token (Read access) from your HuggingFace account <button on:click|preventDefault={() => openLink('https://huggingface.co/settings/tokens')} class="text-blue-600 hover:underline dark:text-blue-400 font-medium">settings</button>.</p>
                        </li>

                        <!-- Step 4: Verify Token -->
                        <li class="mb-10 ms-8">
                            <span class="absolute flex items-center justify-center w-8 h-8 rounded-full -start-4 ring-4 ring-white dark:ring-gray-900 {diarizationAccessGranted || diarizationDownloaded ? 'bg-green-100 text-green-600 dark:bg-green-900/30 dark:text-green-400' : 'bg-blue-100 text-blue-600 dark:bg-blue-900/30 dark:text-blue-400'}">
                                <span class="font-medium text-sm">4</span>
                            </span>
                            <h3 class="font-medium leading-tight text-gray-900 dark:text-white mb-2">Save Token to Harvey</h3>
                            <div class="flex space-x-2 max-w-md">
                                <input 
                                    type="password" 
                                    bind:value={hfToken} 
                                    on:input={() => { diarizationAccessGranted = false; diarizationError = ''; }} 
                                    placeholder="hf_..." 
                                    class="flex-grow border rounded-lg px-3 py-1.5 text-sm bg-white dark:bg-gray-800 border-gray-200 dark:border-gray-700 focus:ring-2 focus:ring-blue-500 outline-none text-gray-900 dark:text-gray-100" 
                                    autocomplete="off"
                                    autocorrect="off"
                                />
                                <button 
                                    on:click={verifyHfToken} 
                                    disabled={!hfToken || isVerifyingToken || isDownloadingDiarization || diarizationAccessGranted} 
                                    class="px-4 py-1.5 bg-gray-900 dark:bg-gray-100 text-white dark:text-gray-900 rounded-lg text-sm font-bold disabled:opacity-50 flex items-center transition-colors"
                                >
                                    {#if isVerifyingToken}<Loader2 class="w-3.5 h-3.5 animate-spin mr-1.5" />{/if}
                                    {diarizationAccessGranted ? 'Verified' : 'Verify'}
                                </button>
                            </div>
                            {#if diarizationError && !isDownloadingDiarization}
                                <p class="text-[10px] text-red-600 dark:text-red-400 font-medium mt-2 flex items-center"><AlertTriangle class="w-3 h-3 mr-1" /> {diarizationError}</p>
                            {/if}
                        </li>

                        <!-- Step 5: Download Model -->
                        <li class="ms-8">
                            <span class="absolute flex items-center justify-center w-8 h-8 rounded-full -start-4 ring-4 ring-white dark:ring-gray-900 {diarizationDownloaded ? 'bg-green-100 text-green-600 dark:bg-green-900/30 dark:text-green-400' : (diarizationAccessGranted ? 'bg-blue-100 text-blue-600 dark:bg-blue-900/30 dark:text-blue-400' : 'bg-gray-100 text-gray-600 dark:bg-gray-800 dark:text-gray-400')}">
                                <span class="font-medium text-sm">5</span>
                            </span>
                            <h3 class="font-medium leading-tight text-gray-900 dark:text-white mb-2">Download Model</h3>

                            {#if diarizationDownloaded}
                                <div class="flex items-center space-x-2 text-green-600 dark:text-green-400 text-sm font-bold" in:fade>
                                    <Check class="w-4 h-4" />
                                    <span>Model is successfully downloaded.</span>
                                </div>
                            {:else}
                                <p class="text-xs text-gray-600 dark:text-gray-400 mb-3">Once authenticated and approved, download the model to your machine.</p>
                                <button 
                                    on:click={downloadDiarization} 
                                    disabled={!diarizationAccessGranted || isDownloadingDiarization} 
                                    class="px-6 py-2 bg-blue-600 text-white text-sm font-bold rounded-lg hover:bg-blue-700 disabled:opacity-50 flex items-center space-x-2 shadow-lg transition-all"
                                >
                                    {#if isDownloadingDiarization}<Loader2 class="w-4 h-4 animate-spin mr-2" />{:else}<Download class="w-4 h-4" />{/if}
                                    <span>{isDownloadingDiarization ? 'Downloading...' : 'Download Model'}</span>
                                </button>
                            {/if}

                            {#if diarizationLogs.length > 0}
                                <div class="mt-4 bg-gray-900 rounded-lg p-4 font-mono text-[10px] text-gray-300 h-40 overflow-y-auto scrollbar-hide border border-gray-800">
                                    {#each diarizationLogs as log}
                                        <div class="mb-1 opacity-80"><span class="text-blue-500 mr-2">›</span>{log.message}</div>
                                    {/each}
                                    {#if isDownloadingDiarization}
                                        <div class="flex items-center text-blue-400 mt-1">
                                            <Loader2 class="w-3 h-3 animate-spin mr-2" />
                                            <span>Processing...</span>
                                        </div>
                                    {/if}
                                </div>
                            {/if}
                        </li>
                    </ol>
                {/if}
            </div>
        {/if}    </div>

    <!-- Footer Area -->
    <svelte:fragment slot="footer">
        <button on:click={prevStep} disabled={currentStep === 1 || isInstalling || isDownloadingDiarization || isCleaningUp || currentStep === 3 || (currentStep === 2 && installProgress.phase === 'complete')} class="px-4 py-2 text-sm font-bold text-gray-600 dark:text-gray-400 disabled:opacity-30 flex items-center transition-colors hover:text-gray-900 dark:hover:text-gray-200">
            <ChevronLeft class="w-4 h-4 mr-1" /> Back
        </button>
        <div class="flex space-x-3">
            {#if currentStep === 2}
                <button on:click={nextStep} disabled={installProgress.phase !== 'complete' || isCleaningUp} class="px-6 py-2 bg-blue-600 text-white rounded-lg font-bold disabled:opacity-50 flex items-center hover:bg-blue-700 transition-colors">
                    Next <ChevronRight class="w-4 h-4 ml-1" />
                </button>
            {:else if currentStep < 7}
                <button on:click={nextStep} disabled={isCleaningUp} class="px-6 py-2 bg-blue-600 text-white rounded-lg font-bold flex items-center hover:bg-blue-700 transition-colors">
                    Next <ChevronRight class="w-4 h-4 ml-1" />
                </button>
            {:else if currentStep === 7}
                <button on:click={nextStep} disabled={installProgress.phase !== 'complete' || isCleaningUp} class="px-6 py-2 bg-blue-600 text-white rounded-lg font-bold disabled:opacity-50 flex items-center hover:bg-blue-700 transition-colors">
                    Next <ChevronRight class="w-4 h-4 ml-1" />
                </button>
            {:else}
                <button on:click={close} disabled={isDownloadingDiarization || isCleaningUp} class="px-8 py-2 bg-green-600 text-white rounded-lg font-bold disabled:opacity-50 hover:bg-green-700 transition-colors">
                    {diarizationDownloaded ? 'Finish' : 'Get Started'}
                </button>
            {/if}
        </div>
    </svelte:fragment>
</Modal>

<style>
    .scrollbar-hide::-webkit-scrollbar { display: none; }
    .scrollbar-hide { -ms-overflow-style: none; scrollbar-width: none; }
    .line-clamp-1 { display: -webkit-box; -webkit-line-clamp: 1; -webkit-box-orient: vertical; overflow: hidden; }
</style>
