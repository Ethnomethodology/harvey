<script>
	import { createEventDispatcher, onMount, onDestroy } from 'svelte';
	import { get } from 'svelte/store';
	import { CheckCircle, XCircle, Clock, Loader } from 'lucide-svelte';
	import { invoke } from '@tauri-apps/api/core';
	import Dropdown from '$lib/components/shared/Dropdown.svelte';
	import { languageMap } from '$lib/constants/languageMap.js';
	import { transcriptStore, setRanTranslationInBackground, clearTranslationStatus } from '$lib/stores/transcriptStore.js';
	import { configStatus } from '$lib/stores/configStatusStore.js';
	import { getSelectedTranslationFamily } from '$lib/services/configureActions';
	import { message } from '@tauri-apps/plugin-dialog';

	export let availableTranscripts = [];
	export let activeTranscriptPath = null;

	const dispatch = createEventDispatcher();

	let selectedTranscript = '';
	let localModels = [];
	let filteredModels = [];
	let modelOptions = [];
	let selectedModel = '';
	let selectedFamily = 'helsinki';
	let selectedSourceLanguage = 'auto';
	let selectedTargetLanguage = 'en';
	
	function formatDuration(seconds) {
		if (!seconds && seconds !== 0) return '0s';
		const m = Math.floor(seconds / 60);
		const s = Math.floor(seconds % 60);
		if (m === 0) return `${s}s`;
		return `${m}m ${s}s`;
	}

	let elapsedText = '';
	let timerInterval;

	function updateElapsed() {
		if ($transcriptStore.isTranslating && $transcriptStore.translationStartTime) {
			const now = Date.now();
			const diff = Math.floor((now - $transcriptStore.translationStartTime) / 1000);
			elapsedText = formatDuration(diff);
		} else {
			elapsedText = '';
		}
	}

	$: if ($transcriptStore.isTranslating && $transcriptStore.translationStartTime) {
		if (!timerInterval) {
			updateElapsed();
			timerInterval = setInterval(updateElapsed, 1000);
		}
	} else {
		if (timerInterval) {
			clearInterval(timerInterval);
			timerInterval = null;
		}
	}

	onDestroy(() => {
		if (timerInterval) clearInterval(timerInterval);
	});

	// Mapping for NLLB language options (based on supported list)
	const nllbLanguageOptions = [
		{ value: 'auto', label: 'Auto-detect' },
		...Array.from(languageMap.entries())
			.filter(([code, name]) => code.length === 2) // Keep standard 2-letter codes
			.map(([code, name]) => ({ value: code, label: name }))
			.sort((a, b) => a.label.localeCompare(b.label))
	];

	// Filtered list for target (remove auto-detect)
	const nllbTargetLanguageOptions = nllbLanguageOptions.filter(opt => opt.value !== 'auto');

	// Warning icon logic (same as ProjectView)
	$: hasCriticalConfigIssues = !$configStatus.python_libraries_installed;
	$: hasNonCriticalConfigIssues =
		!hasCriticalConfigIssues &&
		(!$configStatus.hf_token_present ||
			!$configStatus.transcription_models_downloaded ||
			!$configStatus.diarization_model_downloaded ||
			!$configStatus.translation_models_downloaded);

	function handleOpenConfig() {
		dispatch('openConfig');
	}

	function formatModelDisplayName(modelName) {
		const parts = modelName.split('/');
		const baseName = parts[parts.length - 1] || modelName;

		if (baseName.toLowerCase().includes('nllb')) {
			if (baseName.includes('600M')) return "NLLB-200 Distilled (Small & Fast)";
			if (baseName.includes('1.3B')) return "NLLB-200 (Medium)";
			if (baseName.includes('3.3B')) return "NLLB-200 (Large)";
			return baseName;
		}

		if (parts.length === 2) {
			const langParts = parts[1].split('-');
			if (langParts.length >= 3 && langParts[0] === 'opus' && langParts[1] === 'mt') {
				const fromCode = langParts[langParts.length - 2];
				const toCode = langParts[langParts.length - 1];
				const fromLang = languageMap.get(fromCode);
				const toLang = languageMap.get(toCode);
				if (fromLang && toLang) {
					return `${fromLang} to ${toLang}`;
				}
			}
		}
		return modelName;
	}

	let transcriptOptions = [];
	$: if (availableTranscripts.length > 0) {
		transcriptOptions = availableTranscripts.map(t => ({
			value: t.relativePath,
			label: t.name || t.relativePath
		}));

		const activeTranscript = availableTranscripts.find(t => t.path === activeTranscriptPath);
		if (activeTranscript) {
			selectedTranscript = activeTranscript.relativePath;
		} else if (!selectedTranscript || !availableTranscripts.some(t => t.relativePath === selectedTranscript)) {
			selectedTranscript = availableTranscripts[0].relativePath;
		}
	} else {
		transcriptOptions = [];
		selectedTranscript = '';
	}

	$: {
		filteredModels = localModels.filter(m => m.family === selectedFamily);
		if (filteredModels.length > 0) {
			modelOptions = filteredModels.map(model => ({
				value: model.name,
				label: formatModelDisplayName(model.name)
			}));
			if (!selectedModel || !filteredModels.some(m => m.name === selectedModel)) {
				selectedModel = filteredModels[0]?.name || '';
			}
		} else {
			modelOptions = [];
			selectedModel = '';
		}
	}

	$: if (showModal) {
		loadData();
	}

	async function loadData() {
		try {
			[localModels, selectedFamily] = await Promise.all([
				invoke('get_local_translation_models'),
				getSelectedTranslationFamily()
			]);
			selectedFamily = selectedFamily || 'helsinki';

			// Smarter default for source language
			const activeTranscript = availableTranscripts.find(t => t.path === activeTranscriptPath);
			if (activeTranscript && activeTranscript.language_code) {
				selectedSourceLanguage = activeTranscript.language_code;
			} else {
				selectedSourceLanguage = 'auto';
			}
		} catch (e) {
			console.error("Failed to fetch local translation data:", e);
		}
	}

	onMount(async () => {
		await loadData();
	});

	function handleConfirm() {
		const selectedTranscriptObject = availableTranscripts.find(t => t.relativePath === selectedTranscript);
		
		let sourceLang = 'auto';
		let targetLang = 'auto';

		if (selectedModel.toLowerCase().includes('nllb')) {
			sourceLang = selectedSourceLanguage;
			targetLang = selectedTargetLanguage;
		} else if (selectedModel.includes('-')) {
			const parts = selectedModel.split('-');
			// For Helsinki models like "Helsinki-NLP/opus-mt-en-hi", parts are ["Helsinki-NLP/opus", "mt", "en", "hi"]
			targetLang = parts[parts.length - 1];
			// Source is second to last part
			sourceLang = parts[parts.length - 2] || 'auto';
		}

		dispatch('confirm', {
			transcript: selectedTranscriptObject,
			model: selectedModel,
			sourceLanguage: sourceLang,
			targetLanguage: targetLang
		});
	}

	function handleCancelRequest() {
		dispatch('cancelRequest');
	}

	function handleCloseAndReset() {
		clearTranslationStatus(); // Reset translation-related states
		dispatch('closeAndReset');
	}

	function handleRunInBackgroundAndClose() {
        setRanTranslationInBackground(true);
		dispatch('runInBackgroundAndClose');
	}

	$: showModal = $transcriptStore.showTranslateModal;
	$: isTranslating = $transcriptStore.isTranslating;
	$: jobStatus = $transcriptStore.translationJobStatus;
	$: progressMessage = $transcriptStore.translationProgress.message;
	$: currentErrorMessage = $transcriptStore.translationErrorMessage;

	$: modalTitle = (!isTranslating && jobStatus === null) ? 'Translate Transcript' :
					 (isTranslating && jobStatus === 'initiating') ? 'Initiating Translation...' :
					 (isTranslating && jobStatus === 'running') ? 'Translation Status' :
					 (jobStatus === 'cancelling') ? 'Cancelling Job' :
					 (!isTranslating && jobStatus === 'done') ? 'Translation Complete' :
					 (!isTranslating && jobStatus === 'error') ? 'Translation Error' :
					 (!isTranslating && jobStatus === 'cancelled') ? 'Translation Cancelled' :
					 'Translation Status';

	let durationText = '';

	// Watch for completion to calculate duration
	$: if (!isTranslating && jobStatus === 'done') {
		const endTime = Date.now();
		const startTime = $transcriptStore.translationStartTime;
		const durationMs = startTime ? endTime - startTime : 0;
		const seconds = Math.floor(durationMs / 1000);
		const minutes = Math.floor(seconds / 60);
		const remainingSeconds = seconds % 60;
		
		if (minutes > 0) {
			durationText = `${minutes}m ${remainingSeconds}s`;
		} else {
			durationText = `${seconds}s`;
		}
	}

	function handleKeydown(event) {
		if (showModal && event.key === 'Escape') {
			if (isTranslating && (jobStatus === 'running' || jobStatus === 'initiating')) {
				event.preventDefault();
				return;
			} else {
				handleCloseAndReset();
			}
		}
	}
</script>

{#if showModal}
    <div
        class="fixed inset-0 z-[120] flex items-center justify-center bg-black/50 backdrop-blur-sm"
        role="dialog"
        aria-modal="true"
        aria-labelledby="translate-modal-title"
        on:click={handleCloseAndReset}
        tabindex="-1"
        on:keydown={handleKeydown}
    >
        <div
            class="bg-white dark:bg-gray-900 rounded-lg shadow-xl p-6 w-full max-w-md text-gray-800 dark:text-gray-200 flex flex-col"
            role="document"
            tabindex="-1"
            on:click|stopPropagation
        >
            <h2 id="translate-modal-title" class="text-lg font-semibold mb-4 text-center">{modalTitle}</h2>

            {#if !isTranslating && jobStatus === null}
                <!-- CONFIRM VIEW -->
                <div class="space-y-4 mb-6">
                    <div class="space-y-1">
                        <label for="transcriptSelect" class="block font-medium text-gray-900 dark:text-gray-100">Transcript to Translate:</label>
                        <Dropdown
                            containerClasses="w-full"
                            options={transcriptOptions}
                            bind:value={selectedTranscript}
                            placeholder="Select a Transcript"
                            disabled={availableTranscripts.length === 0}
                        />
                    </div>
                    <div class="space-y-1">
                        {#if localModels.length === 0}
							<div
								class="bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 p-4 rounded-md text-center space-y-3 my-4"
							>
								<p class="text-blue-800 dark:text-blue-300 font-medium">No translation models available.</p>
								<div class="flex items-center justify-center space-x-2">
									<p class="text-xs text-blue-600 dark:text-blue-400">Please download a model in the</p>
									<button
										type="button"
										on:click={handleOpenConfig}
										class="flex items-center space-x-1 bg-gray-100 dark:bg-gray-800 px-2 py-1 rounded border border-gray-300 dark:border-gray-600 shadow-sm hover:bg-gray-200 dark:hover:bg-gray-700 transition-colors"
									>
										<svg
											xmlns="http://www.w3.org/2000/svg"
											viewBox="0 0 24 24"
											fill="currentColor"
											class="w-4 h-4"
											class:text-red-500={hasCriticalConfigIssues}
											class:text-yellow-500={hasNonCriticalConfigIssues}
										>
											<path
												d="M17.004 10.407c.138.435-.216.842-.672.842h-3.465a.75.75 0 0 1-.65-.375l-1.732-3c-.229-.396-.053-.907.393-1.004a5.252 5.252 0 0 1 6.126 3.537ZM8.12 8.464c.307-.338.838-.235 1.066.16l1.732 3a.75.75 0 0 1 0 .75l-1.732 3c-.229.397-.76.5-1.067.161A5.23 5.23 0 0 1 6.75 12a5.23 5.23 0 0 1 1.37-3.536ZM10.878 17.13c-.447-.098-.623-.608-.394-1.004l1.733-3.002a.75.75 0 0 1 .65-.375h3.465c.457 0 .81.407.672.842a5.252 5.252 0 0 1-6.126 3.539Z"
											/>
											<path
												fill-rule="evenodd"
												d="M21 12.75a.75.75 0 1 0 0-1.5h-.783a8.22 8.22 0 0 0-.237-1.357l.734-.267a.75.75 0 1 0-.513-1.41l-.735.268a8.24 8.24 0 0 0-.689-1.192l.6-.503a.75.75 0 1 0-.964-1.149l-.6.504a8.3 8.3 0 0 0-1.054-.885l.391-.678a.75.75 0 1 0-1.299-.75l-.39.676a8.188 8.188 0 0 0-1.295-.47l.136-.77a.75.75 0 0 0-1.477-.26l-.136.77a8.36 8.36 0 0 0-1.377 0l-.136-.77a.75.75 0 1 0-1.477.26l.136.77c-.448.121-.88.28-1.294.47l-.39-.676a.75.75 0 0 0-1.3.75l.392.678a8.29 8.29 0 0 0-1.054.885l-.6-.504a.75.75 0 1 0-.965 1.149l.6.503a8.243 8.243 0 0 0-.689 1.192L3.8 8.216a.75.75 0 1 0-.513 1.41l.735.267a8.222 8.222 0 0 0-.238 1.356h-.783a.75.75 0 0 0 0 1.5h.783c.042.464.122.917.238 1.356l-.735.268a8.24 8.24 0 0 0 .513 1.41l.735-.268c.197.417.428.816.69 1.191l-.6.504a.75.75 0 0 0 .963 1.15l.601-.505c.326.323.679.62 1.054.885l-.392.68a.75.75 0 0 0 1.3.75l.39-.679c.414.192.847.35 1.294.471l-.136.77a.75.75 0 0 0 1.477.261l.137-.772a8.332 8.332 0 0 0 1.376 0l.136.772a.75.75 0 1 0 1.477-.26l-.136-.771a8.19 8.19 0 0 0 1.294-.47l.391.677a.75.75 0 0 0 1.3-.75l-.393-.679a8.29 8.29 0 0 0 1.054-.885l.601.504a.75.75 0 1 0-.965 1.149l.6.503a8.243 8.243 0 0 0-.689 1.192L18.2 15.784a.75.75 0 1 0 .513-1.41l.735-.267a8.222 8.222 0 0 0 .237-1.356h.784Zm-2.657-3.06a6.744 6.744 0 0 0-1.19-2.053 6.784 6.784 0 0 0-1.82-1.51A6.705 6.705 0 0 0 12 5.25a6.8 6.8 0 0 0-1.225.11 6.7 6.7 0 0 0-2.15.793 6.784 6.784 0 0 0-2.952 3.489.76.76 0 0 1-.036.098A6.74 6.74 0 0 0 5.251 12a6.74 6.74 0 0 0 3.366 5.842l.009.005a6.704 6.704 0 0 0 2.18.798l.022.003a6.792 6.792 0 0 0 2.368-.004 6.704 6.704 0 0 0 2.205-.811 6.785 6.785 0 0 0 1.762-1.484l.009-.01.009-.01a6.743 6.743 0 0 0 1.18-2.066c.253-.707.39-1.469.39-2.263a6.74 6.74 0 0 0-.408-2.309Z"
												clip-rule="evenodd"
											/>
										</svg>
										<span class="text-xs font-semibold text-gray-700 dark:text-gray-200">Configure</span>
									</button>
									<p class="text-xs text-blue-600 dark:text-blue-400">screen.</p>
								</div>
							</div>
						{:else}
							<label for="modelSelect" class="block font-medium text-gray-900 dark:text-gray-100">Translation Model:</label>
							<Dropdown
								containerClasses="w-full"
								options={modelOptions}
								bind:value={selectedModel}
								placeholder="Select a Model"
							/>
						{/if}
                    </div>

					{#if selectedModel.toLowerCase().includes('nllb')}
						<div class="grid grid-cols-2 gap-4">
							<div class="space-y-1">
								<label class="block font-medium text-gray-900 dark:text-gray-100">From:</label>
								<Dropdown
									containerClasses="w-full"
									options={nllbLanguageOptions}
									bind:value={selectedSourceLanguage}
									placeholder="Source"
								/>
							</div>
							<div class="space-y-1">
								<label class="block font-medium text-gray-900 dark:text-gray-100">To:</label>
								<Dropdown
									containerClasses="w-full"
									options={nllbTargetLanguageOptions}
									bind:value={selectedTargetLanguage}
									placeholder="Target"
								/>
							</div>
						</div>
					{/if}
                </div>
                <div class="flex justify-end space-x-3 mt-auto pt-4 border-t border-gray-200 dark:border-gray-700">
                    <button class="btn-secondary" on:click={handleCloseAndReset}>Cancel</button>
                    <button class="btn-primary" on:click={handleConfirm} disabled={availableTranscripts.length === 0 || !selectedTranscript || !selectedModel}>Start Translation</button>
                </div>

            {:else if isTranslating && (jobStatus === 'running' || jobStatus === 'initiating')}
				<!-- RUNNING OR INITIATING VIEW -->
				<div class="flex flex-col items-center space-y-4 mb-6">
                    <div class="w-16 h-16">
                        <Loader class="w-full h-full text-blue-500 animate-spin" />
                    </div>
					<p class="text-xs text-center text-gray-600 dark:text-gray-400 h-4">
                        {progressMessage || (jobStatus === 'initiating' ? 'Preparing...' : 'Processing...')}
					</p>
					{#if elapsedText}
						<p class="text-xs text-center text-gray-500 dark:text-gray-500 font-mono mt-1">
							{elapsedText}
						</p>
					{/if}
				</div>
				<div class="flex justify-center space-x-2 mt-auto">
					<button class="btn-secondary" on:click={handleRunInBackgroundAndClose} disabled={jobStatus === 'initiating'}>
						Run in background
					</button>
					<button class="btn-action-cancel" on:click={handleCancelRequest} disabled={jobStatus === 'initiating'}>
						Request Cancellation
					</button>
				</div>

			{:else if jobStatus === 'cancelling'}
				<!-- CANCELLING VIEW -->
				<div class="flex flex-col items-center space-y-4 mb-6">
                    <div class="w-16 h-16">
                        <Clock class="w-full h-full text-orange-500" />
                    </div>
					<p class="text-xs text-center text-gray-600 dark:text-gray-400 h-4">
						{progressMessage || 'Attempting to cancel...'}
					</p>
				</div>
				<div class="flex justify-center space-x-2 mt-auto">
					<button class="btn-secondary" disabled>Cancelling...</button>
				</div>

			{:else if !isTranslating && jobStatus === 'done'}
				<!-- DONE VIEW -->
				<div class="flex flex-col items-center space-y-3 mb-6 text-center">
					<CheckCircle class="w-16 h-16 text-green-500" />
					<p class="text-sm font-medium">Translation Complete!</p>
					{#if durationText}
						<p class="text-xs text-gray-500 dark:text-gray-400">Time taken: {durationText}</p>
					{/if}
				</div>
				<div class="flex justify-center mt-auto">
					<button class="btn-primary" on:click={handleCloseAndReset}>Close</button>
				</div>

			{:else if !isTranslating && jobStatus === 'cancelled'}
                <!-- CANCELLED VIEW -->
				<div class="flex flex-col items-center space-y-3 mb-6 text-center">
					<XCircle class="w-16 h-16 text-orange-500" />
					<p class="text-sm font-medium">{progressMessage || 'Translation Cancelled'}</p>
				</div>
				<div class="flex justify-center mt-auto">
					<button class="btn-secondary" on:click={handleCloseAndReset}>Close</button>
				</div>

			{:else if !isTranslating && jobStatus === 'error'}
				<!-- ERROR VIEW -->
				<div class="flex flex-col items-center space-y-3 mb-6 text-center">
					<XCircle class="w-16 h-16 text-red-500" />
					<p class="text-sm font-medium">An Error Occurred</p>
					<p class="text-xs bg-red-100 dark:bg-red-900/50 border border-red-300 dark:border-red-700 text-red-700 dark:text-red-300 p-2 rounded w-full text-left overflow-x-auto max-h-32">
						{currentErrorMessage || 'Unknown error during translation.'}
					</p>
				</div>
				<div class="flex justify-center mt-auto">
					<button class="btn-secondary" on:click={handleCloseAndReset}>Close</button>
				</div>
			{:else}
				<!-- Fallback or initial brief loading state -->
				<div class="flex flex-col items-center space-y-4 py-8">
					<Loader class="w-12 h-12 text-gray-400 animate-spin" />
					<p class="text-sm text-gray-500">Loading status...</p>
				</div>
			{/if}
        </div>
    </div>
{/if}

<style>
	.btn-primary,
	.btn-secondary,
	.btn-action-cancel {
		padding: 0.5rem 1rem;
		border: none;
		border-radius: 0.375rem; /* 6px */
		cursor: pointer;
		font-size: 0.875rem; /* 14px */
		font-weight: 500;
		transition: background-color 0.15s ease-in-out, opacity 0.15s ease-in-out;
		white-space: nowrap;
		display: inline-flex;
		align-items: center;
		justify-content: center;
	}
	.btn-primary {
		background-color: #3b82f6;
		color: white;
	}
	.btn-primary:hover:not(:disabled) {
		background-color: #2563eb;
	}
	.btn-primary:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}
	.btn-secondary {
		background-color: #e5e7eb;
		color: #374151;
		border: 1px solid #d1d5db;
	}
	.dark .btn-secondary {
		background-color: #4b5563;
		color: #e5e7eb;
		border-color: #6b7280;
	}
	.btn-secondary:hover:not(:disabled) {
		background-color: #d1d5db;
	}
	.dark .btn-secondary:hover:not(:disabled) {
		background-color: #6b7280;
	}
	.btn-secondary:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}
	.btn-action-cancel {
		background-color: #ef4444;
		color: white;
	}
	.btn-action-cancel:hover:not(:disabled) {
		background-color: #dc2626;
	}
	.btn-action-cancel:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}
    .dark .btn-action-cancel {
         background-color: #dc2626;
    }
    .dark .btn-action-cancel:hover:not(:disabled) {
        background-color: #b91c1c;
    }
</style>
