<!-- src/lib/components/projectview/modals/TranscribeConfirmModal.svelte -->
<script>
	import { createEventDispatcher, onDestroy } from 'svelte';
	import { get } from 'svelte/store';
	import { CheckCircle, XCircle, Clock, Loader } from 'lucide-svelte';
	import { transcriptStore } from '$lib/stores/transcriptStore.js';
	import { configStatus } from '$lib/stores/configStatusStore.js';
	import SpeakersModal from './SpeakersModal.svelte';
	import Dropdown from '$lib/components/shared/Dropdown.svelte';
	import AdditionalParametersModal from './AdditionalParametersModal.svelte';
	import { invoke } from '@tauri-apps/api/core';
	import { project as projectMainStore } from '$lib/stores/projectStore.js';

	// Props
	export let fileName = '';
	export let downloadedModelsList = [];
	export let languageOptions = []; // Expect languageOptions as a prop, default to empty array
	export let speakers = { count: 0, names: [], translatedNames: [] };
	export let mediaDuration = 0; // Added for manual mode validation
	export let lastSegmentEndTime = 0; // Added for manual mode validation

	const dispatch = createEventDispatcher();

	// Local state for editable fields
	let modalSelectedModel = '';
	let modalTranscriptionMode = 'automatic'; // Kept for store compatibility if needed, but UI uses modalTab
	let modalTab = 'automatic'; // 'automatic' | 'manual'
	let modalSelectedLanguage = 'auto';
    let modalInitialPrompt = '';
    let modalHotwords = '';
    let showAdditionalParamsModal = false;

	let modalEnableDiarization = false;
	let modalSpeakersConfig = { count: 0, names: [], translatedNames: [] };
	let showNestedSpeakersModal = false;

	// Manual Mode State
	let manualSegmentCount = 1;
	let manualSegmentDuration = 60;
	let manualSpeakerMode = 'unassigned';

    function handleManualSegCountInput(e) {
        const val = parseInt(e.target.value);
        if (!isNaN(val) && val > 0 && mediaDuration > 0) {
            manualSegmentDuration = Math.max(1, Math.round(mediaDuration / val));
        }
    }

    function handleManualSegDurationInput(e) {
        const val = parseInt(e.target.value);
        if (!isNaN(val) && val > 0 && mediaDuration > 0) {
            manualSegmentCount = Math.min(100, Math.max(1, Math.round(mediaDuration / val)));
        }
    }

	// Derived state for manual validation
	$: totalDurationNeeded = manualSegmentCount * manualSegmentDuration;
	// For manual transcription initialization (from this modal), we treat it as creating a new transcript/overwriting.
	// So we validate against total media duration, not remaining space.
	$: isManualDurationValid = totalDurationNeeded <= mediaDuration + 0.001;

	const manualSpeakerOptions = [
		{ value: 'unassigned', label: 'Unassigned' },
		{ value: 'alternate', label: 'Alternate Speakers', disabled: (speakers?.names?.length || 0) < 2 },
	];

	function formatDuration(seconds) {
		if (!seconds && seconds !== 0) return '0s';
		const m = Math.floor(seconds / 60);
		const s = Math.floor(seconds % 60);
		if (m === 0) return `${s}s`;
		return `${m}m ${s}s`;
	}

	let durationText = '';
	let elapsedText = '';
	let timerInterval;

	function updateElapsed() {
		if ($transcriptStore.isTranscribing && $transcriptStore.transcriptionStartTime) {
			const now = Date.now();
			const diff = Math.floor((now - $transcriptStore.transcriptionStartTime) / 1000);
			elapsedText = formatDuration(diff);
		} else {
			elapsedText = '';
		}
	}

	$: if ($transcriptStore.isTranscribing && $transcriptStore.transcriptionStartTime) {
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

	// Event Handlers
	function handleConfirm() {
		// Start time is now handled by store
		if (modalTab === 'automatic') {
            const selectedModelObj = downloadedModelsList.find(m => m.name === modalSelectedModel);
            const engine = selectedModelObj?.family || 'whisper-cpp';

			dispatch('confirmStart', {
				transcriptionMode: 'automatic',
				selectedModel: modalSelectedModel,
                selectedTranscriptionEngine: engine,
				selectedLanguage: modalSelectedLanguage,
				enableDiarization: modalEnableDiarization,
				speakersConfig: modalSpeakersConfig,
                initialPrompt: modalInitialPrompt,
                hotwords: modalHotwords
			});
		} else {
			dispatch('confirmStart', {
				transcriptionMode: 'manual',
				manualSettings: {
					segmentCount: manualSegmentCount,
					segmentDuration: manualSegmentDuration,
					speakerMode: manualSpeakerMode,
					startTime: 0, // Always start from 0 for new transcript via this modal
				},
				speakersConfig: modalSpeakersConfig, // Pass speakers config even in manual for alternations
			});
		}
	}

	function handleCancelRequest() {
		dispatch('cancelRequest');
	}

	function handleCloseAndReset() {
		dispatch('closeAndReset');
	}

	function handleRunInBackgroundAndClose() {
		dispatch('runInBackgroundAndClose');
	}

	function handleOpenConfig() {
		dispatch('openConfig');
	}

	// --- Reactive Derivations from Store for UI ---
	$: showModal = $transcriptStore.showTranscribeModal;
	$: isTranscribing = $transcriptStore.isTranscribing;
	$: jobStatus = $transcriptStore.transcriptionJobStatus;
	$: progressPercent = $transcriptStore.transcriptionProgress.percent;
	$: progressMessage = $transcriptStore.transcriptionProgress.message;
	$: currentErrorMessage = $transcriptStore.transcriptionErrorMessage;
	$: currentJobId = $transcriptStore.transcriptionJobId;

	// Warning icon logic (same as ProjectView)
	$: hasCriticalConfigIssues = !$configStatus.python_libraries_installed;
	$: hasNonCriticalConfigIssues =
		!hasCriticalConfigIssues &&
		(!$configStatus.hf_token_present ||
			!$configStatus.transcription_models_downloaded ||
			!$configStatus.diarization_model_downloaded ||
			!$configStatus.translation_models_downloaded);

	let isInitialized = false;
	// When the modal is about to show the confirm view, initialize local states from the store
	$: if (showModal && !isTranscribing && jobStatus === null && !isInitialized) {
		modalSelectedModel =
			$transcriptStore.selectedModelName || (downloadedModelsList.length > 0 ? downloadedModelsList[0].name : '');
		modalSelectedLanguage = $transcriptStore.selectedLanguage || 'auto';
		modalTab = $transcriptStore.transcriptionMode || 'automatic'; // Initialize tab from store

        modalInitialPrompt = $transcriptStore.initialPrompt || '';
        modalHotwords = $transcriptStore.hotwords || '';

		modalEnableDiarization = $transcriptStore.diarizationEnabledForNextJob;
		// Initialize modalSpeakersConfig from the speakers prop (which comes from transcriptStore initially)
		// Use a deep copy to prevent direct mutation of the prop or store value until confirmed.
		modalSpeakersConfig = JSON.parse(JSON.stringify(speakers || { count: 0, names: [], translatedNames: [] }));

		// Initialize manual settings from store if available
		if ($transcriptStore.manualSegmentSettings) {
			manualSegmentDuration = $transcriptStore.manualSegmentSettings.duration || 60;
			manualSpeakerMode = $transcriptStore.manualSegmentSettings.speakerMode || 'unassigned';
			if (manualSpeakerMode === 'unselected') manualSpeakerMode = 'unassigned'; // Migration
		}

		isInitialized = true;
	}

	// Update modalSelectedModel if downloadedModelsList changes and it's empty
	$: if (showModal && !modalSelectedModel && downloadedModelsList.length > 0) {
		modalSelectedModel = downloadedModelsList[0].name;
	}

	// Reset the initialization flag when the modal is closed
	$: if (!showModal) {
		isInitialized = false;
	}

	// --- Title Logic ---
	$: modalTitle =
		!isTranscribing && jobStatus === null
			? 'Transcription Settings'
			: isTranscribing && jobStatus === 'initiating'
				? 'Initiating Transcription...'
				: isTranscribing && jobStatus === 'running'
					? 'Transcription Status'
					: jobStatus === 'cancelling'
						? 'Cancelling Job'
						: !isTranscribing && jobStatus === 'done'
							? 'Transcription Complete'
							: !isTranscribing && jobStatus === 'error'
								? 'Transcription Error'
								: !isTranscribing && jobStatus === 'cancelled'
									? 'Transcription Cancelled'
									: 'Transcription Status';

	// Watch for completion to calculate duration
	$: if (!isTranscribing && jobStatus === 'done') {
		const endTime = Date.now();
		const startTime = $transcriptStore.transcriptionStartTime;
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

	// Keyboard handling (optional, can be simplified or removed if not strictly needed by new design)
	function handleKeydown(event) {
		if (showModal && event.key === 'Escape') {
			// If transcribing and running/initiating, do nothing on Escape.
			if (isTranscribing && (jobStatus === 'running' || jobStatus === 'initiating')) {
				// Explicitly do nothing to prevent closure
				event.preventDefault(); // Also prevent any default browser behavior for Escape if modal is focused
				return;
			} else if (
				!isTranscribing &&
				(jobStatus === 'done' || jobStatus === 'error' || jobStatus === 'cancelled' || jobStatus === null)
			) {
				// For terminal states or initial confirm state, allow close.
				handleCloseAndReset();
			}
			// Other states (e.g., 'cancelling') will also do nothing, which is fine.
		}
	}

	onDestroy(() => {
		// Global listener cleanup is removed.
	});
</script>

{#if showModal}
	<div
		class="fixed inset-0 z-[120] flex items-center justify-center bg-black/50 backdrop-blur-sm"
		role="dialog"
		aria-modal="true"
		aria-labelledby="transcribe-modal-title"
		on:click={() => {
			if (!(isTranscribing && (jobStatus === 'running' || jobStatus === 'initiating'))) {
				handleCloseAndReset();
			}
		}}
		tabindex="-1"
		on:keydown={handleKeydown}
	>
		<div
			class="bg-white dark:bg-gray-900 rounded-lg shadow-xl p-6 w-full max-w-md text-gray-800 dark:text-gray-200 flex flex-col"
			role="document"
			tabindex="-1"
			on:click|stopPropagation
		>
			<h2 id="transcribe-modal-title" class="text-lg font-semibold mb-4 text-center">{modalTitle}</h2>

			{#if !isTranscribing && jobStatus === null}
				<!-- CONFIRM VIEW -->

				<!-- Tabs -->
				<div class="flex border-b border-gray-200 dark:border-gray-700 mb-4">
					<button
						class="flex-1 py-2 text-sm font-medium text-center border-b-2 focus:outline-none transition-colors {modalTab ===
						'automatic'
							? 'border-blue-500 text-blue-600 dark:text-blue-400'
							: 'border-transparent text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-300'}"
						on:click={() => (modalTab = 'automatic')}
					>
						Automatic
					</button>
					<button
						class="flex-1 py-2 text-sm font-medium text-center border-b-2 focus:outline-none transition-colors {modalTab ===
						'manual'
							? 'border-blue-500 text-blue-600 dark:text-blue-400'
							: 'border-transparent text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-300'}"
						on:click={() => (modalTab = 'manual')}
					>
						Manual
					</button>
				</div>

				<div class="space-y-3 text-sm mb-5 text-gray-700 dark:text-gray-300 max-h-[60vh] overflow-y-auto pr-2">
					<div><strong>File:</strong> <span class="font-mono break-all ml-2">{fileName || 'N/A'}</span></div>

					{#if modalTab === 'automatic'}
						<!-- AUTOMATIC SETTINGS -->
                        {#if hasCriticalConfigIssues}
							<div
								class="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 p-4 rounded-md text-center space-y-3 my-4"
							>
								<p class="text-red-800 dark:text-red-300 font-medium">Required libraries are missing.</p>
                                <p class="text-xs text-red-600 dark:text-red-400">Please install Python dependencies in the Configure screen.</p>
                                <div class="flex justify-center mt-2">
									<button
										type="button"
										on:click={handleOpenConfig}
										class="flex items-center space-x-1 bg-red-100 dark:bg-red-900 px-3 py-1.5 rounded border border-red-300 dark:border-red-600 shadow-sm hover:bg-red-200 dark:hover:bg-red-800 transition-colors text-red-800 dark:text-red-100 text-xs font-semibold"
									>
										Configure
									</button>
                                </div>
							</div>
						{:else if downloadedModelsList.length === 0}
							<div
								class="bg-yellow-50 dark:bg-yellow-900/20 border border-yellow-200 dark:border-yellow-800 p-4 rounded-md text-center space-y-3 my-4"
							>
								<p class="text-yellow-800 dark:text-yellow-300 font-medium">No transcription models available.</p>
                                <div class="flex items-center justify-center space-x-2">
                                    <p class="text-xs text-yellow-700 dark:text-yellow-400">Please download a model in the</p>
									<button
										type="button"
										on:click={handleOpenConfig}
										class="flex items-center space-x-1 bg-gray-100 dark:bg-gray-800 px-2 py-1 rounded border border-gray-300 dark:border-gray-600 shadow-sm hover:bg-gray-200 dark:hover:bg-gray-700 transition-colors"
									>
										<svg
											xmlns="http://www.w3.org/2000/svg"
											viewBox="0 0 24 24"
											fill="currentColor"
											class="w-4 h-4 text-yellow-500"
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
                                    <p class="text-xs text-yellow-700 dark:text-yellow-400">screen.</p>
                                </div>
							</div>
						{:else}
							<div class="space-y-1">
								<label for="modalModelSelect" class="block font-medium text-gray-900 dark:text-gray-100">Model:</label>
								<Dropdown
									containerClasses="w-full"
									options={downloadedModelsList.map((m) => ({ value: m.name, label: `${m.name} (${m.family || 'whisper.cpp'})` }))}
									bind:value={modalSelectedModel}
									placeholder="Select a Model"
									disabled={downloadedModelsList.length === 0}
								/>
							</div>

							<div class="space-y-1">
								<label for="modalLanguageSelect" class="block font-medium text-gray-900 dark:text-gray-100">Language:</label>
								<Dropdown
									containerClasses="w-full"
									options={languageOptions}
									bind:value={modalSelectedLanguage}
									placeholder="Select a Language"
								/>
							</div>

							<div class="pt-2 space-y-3 border-t border-gray-200 dark:border-gray-700 mt-3">
								<div class="flex justify-between items-center">
									<div>
										<strong>Speakers:</strong>
										<span>{modalSpeakersConfig?.count > 0 ? modalSpeakersConfig.count : '0'}</span>
									</div>
									<button type="button" class="btn-xs-secondary" on:click={() => (showNestedSpeakersModal = true)}>
										Edit Speakers
									</button>
								</div>
								{#if modalSpeakersConfig?.count > 0 && modalSpeakersConfig.names && modalSpeakersConfig.names.length > 0}
									<div class="pl-4">
										<p class="text-xs text-gray-500 dark:text-gray-400 break-all">
											({modalSpeakersConfig.names.join(', ')})
										</p>
									</div>
								{/if}

								{#if $configStatus.diarization_model_downloaded}
									<div class="flex items-center space-x-2">
										<input
											type="checkbox"
											id="modalEnableDiarizationCheckbox"
											class="ui-checkbox"
											bind:checked={modalEnableDiarization}
											autocomplete="off"
											autocorrect="off"
										/>
										<label
											for="modalEnableDiarizationCheckbox"
											class="text-sm text-gray-700 dark:text-gray-300 cursor-pointer select-none"
										>
											Identify different speakers (diarize)
										</label>
									</div>
									{#if modalEnableDiarization}
										<p
											class="text-xs mt-1 ml-6 px-2 py-1 rounded bg-yellow-300 text-black dark:bg-yellow-500 dark:text-black"
										>
											Note: Speaker identification can significantly increase transcription time.
										</p>
									{/if}
								{:else}
									<div class="flex flex-col space-y-2">
										<div class="flex items-center justify-between">
											<span class="text-sm text-gray-500 dark:text-gray-400">Speaker identification (diarize)</span>
											<button
												type="button"
												on:click={handleOpenConfig}
												class="flex items-center space-x-1 bg-gray-100 dark:bg-gray-800 px-2 py-1 rounded border border-gray-300 dark:border-gray-600 shadow-sm hover:bg-gray-200 dark:hover:bg-gray-700 transition-colors"
											>
												<svg
													xmlns="http://www.w3.org/2000/svg"
													viewBox="0 0 24 24"
													fill="currentColor"
													class="w-4 h-4 text-yellow-500"
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
										</div>
										<p class="text-[10px] text-gray-500 dark:text-gray-400 italic">
											Download the diarization model in the Configure screen to enable this feature.
										</p>
									</div>
								{/if}
							</div>

                            <div class="pt-2 border-t border-gray-200 dark:border-gray-700 mt-3 flex justify-center">
                                <button type="button" class="btn-xs-secondary w-full" on:click={() => (showAdditionalParamsModal = true)}>
                                    Edit Additional Parameters
                                </button>
                            </div>
						{/if}
					{:else}
						<!-- MANUAL SETTINGS -->
						<div class="space-y-1">
							<label for="manualSegCount" class="block font-medium text-gray-900 dark:text-gray-100"
								>Number of Segments:</label
							>
							<input
								id="manualSegCount"
								type="number"
								min="1"
								max="100"
								bind:value={manualSegmentCount}
                                on:input={handleManualSegCountInput}
								class="ui-input w-full"
							/>
						</div>

						<div class="space-y-1">
							<label for="manualSegDuration" class="block font-medium text-gray-900 dark:text-gray-100"
								>Duration (seconds):</label
							>
							<div class="flex items-center gap-2">
								<input
									id="manualSegDuration"
									type="number"
									min="1"
									bind:value={manualSegmentDuration}
                                    on:input={handleManualSegDurationInput}
									class="ui-input w-full"
								/>
								<span class="text-xs text-gray-500 whitespace-nowrap min-w-[4rem]">
									({formatDuration(manualSegmentDuration)})
								</span>
							</div>
						</div>

						<div
							class="mt-2 p-2 bg-gray-50 dark:bg-gray-800 rounded border border-gray-200 dark:border-gray-700 text-xs space-y-1"
						>
							<div class="flex justify-between">
								<span>Media Duration:</span>
								<span class="font-medium">{formatDuration(mediaDuration)}</span>
							</div>
							<div class="flex justify-between">
								<span>Segmented:</span>
								<span
									class="font-bold {isManualDurationValid
										? 'text-blue-600 dark:text-blue-400'
										: 'text-red-600 dark:text-red-400'}"
								>
									{formatDuration(totalDurationNeeded)}
								</span>
							</div>
							{#if !isManualDurationValid}
								<p
									class="text-center font-semibold text-red-600 dark:text-red-400 pt-1 border-t border-gray-200 dark:border-gray-600"
								>
									Exceeds total media duration!
								</p>
							{/if}
						</div>

						<div class="pt-1 space-y-1 border-t border-gray-200 dark:border-gray-700 mt-3 mb-2">
							<div class="flex justify-between items-center">
								<div>
									<strong>Speakers:</strong>
									<span>{modalSpeakersConfig?.count > 0 ? modalSpeakersConfig.count : '0'}</span>
								</div>
								<button type="button" class="btn-xs-secondary" on:click={() => (showNestedSpeakersModal = true)}>
									Edit Speakers
								</button>
							</div>
							{#if modalSpeakersConfig?.count > 0 && modalSpeakersConfig.names && modalSpeakersConfig.names.length > 0}
								<div class="pl-4">
									<p class="text-xs text-gray-500 dark:text-gray-400 break-all">
										({modalSpeakersConfig.names.join(', ')})
									</p>
								</div>
							{/if}
						</div>

						<div class="space-y-2">
							<span class="block font-medium text-gray-900 dark:text-gray-100">Speaker Assignment:</span>
							<div class="flex flex-col space-y-2 ml-1">
								{#each manualSpeakerOptions as option}
									<label class="flex items-center space-x-2 cursor-pointer {option.disabled ? 'opacity-50 cursor-not-allowed' : ''}">
										<input
											type="radio"
											name="manualSpeakerMode"
											value={option.value}
											bind:group={manualSpeakerMode}
											disabled={option.disabled}
											class="ui-radio"
										/>
										<span class="text-sm text-gray-700 dark:text-gray-300">{option.label}</span>
									</label>
								{/each}
							</div>
							{#if modalSpeakersConfig.names.length < 2}
								<p class="text-xs text-gray-500 mt-1 italic">Note: 'Alternate Speakers' requires at least 2 speakers.</p>
							{:else if manualSpeakerMode === 'alternate' && modalSpeakersConfig.names.length < 2}
								<p class="text-xs text-red-500 mt-1">Need at least 2 speakers configured.</p>
							{/if}
						</div>
					{/if}
				</div>
				<div class="flex justify-end space-x-3 mt-auto pt-4 border-t border-gray-200 dark:border-gray-700">
					<button class="btn-secondary" on:click={handleCloseAndReset}>Cancel</button>
					<button
						class="btn-primary"
						on:click={handleConfirm}
						disabled={(modalTab === 'automatic' && (!modalSelectedModel || !modalSelectedLanguage)) ||
							(modalTab === 'manual' && !isManualDurationValid)}
					>
						{modalTab === 'automatic' ? 'Start Transcription' : 'Add Segments'}
					</button>
				</div>
			{:else if isTranscribing && (jobStatus === 'running' || jobStatus === 'initiating')}
				<!-- RUNNING OR INITIATING VIEW -->
				<div class="flex flex-col items-center space-y-4 mb-6">
					<div class="w-16 h-16">
						<Loader class="w-full h-full text-blue-500 animate-spin" />
					</div>
					<!-- Progress bar removed -->
					<p class="text-xs text-center text-gray-600 dark:text-gray-400 h-4">
						{progressMessage ||
							(jobStatus === 'initiating'
								? 'Preparing...'
								: jobStatus === 'running'
									? 'Processing...'
									: 'Please wait...')}
					</p>
					{#if elapsedText}
						<p class="text-xs text-center text-gray-500 dark:text-gray-500 font-mono mt-1">
							{elapsedText}
						</p>
					{/if}
				</div>
				<div class="flex justify-center space-x-2 mt-auto">
					<button
						class="btn-secondary"
						on:click={handleRunInBackgroundAndClose}
						disabled={jobStatus === 'initiating'}
					>
						Run in background
					</button>
					<button class="btn-action-cancel" on:click={handleCancelRequest} disabled={jobStatus === 'initiating'}>
						Request Cancellation
					</button>
				</div>
			{:else if jobStatus === 'cancelling'}
				<!-- CANCELLING VIEW (Added this state based on logic) -->
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
			{:else if !isTranscribing && jobStatus === 'done'}
				<!-- DONE VIEW -->
				<div class="flex flex-col items-center space-y-3 mb-6 text-center">
					<CheckCircle class="w-16 h-16 text-green-500" />
					<p class="text-sm font-medium">Transcription Complete!</p>
					{#if durationText}
						<p class="text-xs text-gray-500 dark:text-gray-400">Time taken: {durationText}</p>
					{/if}
				</div>
				<div class="flex justify-center mt-auto">
					<button class="btn-primary" on:click={handleCloseAndReset}>Close</button>
				</div>
			{:else if !isTranscribing && jobStatus === 'cancelled'}
				<!-- CANCELLED VIEW -->
				<div class="flex flex-col items-center space-y-3 mb-6 text-center">
					<XCircle class="w-16 h-16 text-orange-500" />
					<p class="text-sm font-medium">{progressMessage || 'Transcription Cancelled'}</p>
				</div>
				<div class="flex justify-center mt-auto">
					<button class="btn-secondary" on:click={handleCloseAndReset}>Close</button>
				</div>
			{:else if !isTranscribing && jobStatus === 'error'}
				<!-- ERROR VIEW -->
				<div class="flex flex-col items-center space-y-3 mb-6 text-center">
					<XCircle class="w-16 h-16 text-red-500" />
					<p class="text-sm font-medium">An Error Occurred</p>
					<p
						class="text-xs bg-red-100 dark:bg-red-900/50 border border-red-300 dark:border-red-700 text-red-700 dark:text-red-300 p-2 rounded w-full text-left overflow-x-auto max-h-32"
					>
						{currentErrorMessage || 'Unknown error during transcription.'}
					</p>
				</div>
				<div class="flex justify-center mt-auto">
					<button class="btn-secondary" on:click={handleCloseAndReset}>Close</button>
				</div>
			{:else}
				<!-- Fallback or initial brief loading state if necessary -->
				<div class="flex flex-col items-center space-y-4 py-8">
					<Loader class="w-12 h-12 text-gray-400 animate-spin" />
					<p class="text-sm text-gray-500">Loading status...</p>
				</div>
			{/if}
		</div>
	</div>
{/if}

{#if showNestedSpeakersModal}
	<SpeakersModal
		bind:showModal={showNestedSpeakersModal}
		currentSpeakers={modalSpeakersConfig}
		on:confirm={(e) => {
			modalSpeakersConfig = e.detail; // Update local config
			showNestedSpeakersModal = false;
		}}
		on:close={() => (showNestedSpeakersModal = false)}
	/>
{/if}

{#if showAdditionalParamsModal}
    <AdditionalParametersModal
        bind:showModal={showAdditionalParamsModal}
        currentEngine={downloadedModelsList.find(m => m.name === modalSelectedModel)?.family || 'whisper-cpp'}
        initialPrompt={modalInitialPrompt}
        hotwords={modalHotwords}
        on:confirm={async (e) => {
            modalInitialPrompt = e.detail.initialPrompt;
            modalHotwords = e.detail.hotwords;

            // 1. Update store
            transcriptStore.update(ts => ({ ...ts, initialPrompt: modalInitialPrompt, hotwords: modalHotwords }));

            // 2. Update DB
            try {
                const projectData = get(projectMainStore);
                const relativePath = $transcriptStore.selectedMediaFile?.relative_path;

                if (projectData && projectData.id && relativePath) {
                    await invoke('save_media_additional_parameters', {
                        projectId: projectData.id,
                        assetRelativePath: relativePath,
                        initialPrompt: modalInitialPrompt,
                        hotwords: modalHotwords
                    });
                }
            } catch (err) {
                console.error("Failed to save additional parameters immediately:", err);
            }

            showAdditionalParamsModal = false;
        }}
        on:close={() => (showAdditionalParamsModal = false)}
    />
{/if}

<style>
	/* Styles remain unchanged */
	.btn-primary,
	.btn-secondary,
	.btn-action-cancel {
		padding: 0.5rem 1rem;
		border: none;
		border-radius: 0.375rem; /* 6px */
		cursor: pointer;
		font-size: 0.875rem; /* 14px */
		font-weight: 500;
		transition:
			background-color 0.15s ease-in-out,
			opacity 0.15s ease-in-out;
		white-space: nowrap;
		display: inline-flex;
		align-items: center;
		justify-content: center;
	}
	.btn-primary {
		background-color: #3b82f6; /* bg-blue-500 */
		color: white;
	}
	.btn-primary:hover:not(:disabled) {
		background-color: #2563eb; /* hover:bg-blue-600 */
	}
	.btn-primary:disabled {
		opacity: 0.6;
		cursor: not-allowed;
		/* background-color: #9ca3af; Let default browser style handle disabled bg */
	}
	.btn-secondary {
		background-color: #e5e7eb; /* bg-gray-200 */
		color: #374151; /* text-gray-700 */
		border: 1px solid #d1d5db; /* border-gray-300 */
	}
	.dark .btn-secondary {
		background-color: #4b5563; /* dark:bg-gray-600 */
		color: #e5e7eb; /* dark:text-gray-200 */
		border-color: #6b7280; /* dark:border-gray-500 */
	}
	.btn-secondary:hover:not(:disabled) {
		background-color: #d1d5db; /* hover:bg-gray-300 */
	}
	.dark .btn-secondary:hover:not(:disabled) {
		background-color: #6b7280; /* dark:hover:bg-gray-500 */
	}
	.btn-secondary:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}
	.btn-action-cancel {
		background-color: #ef4444; /* bg-red-500 */
		color: white;
	}
	.btn-action-cancel:hover:not(:disabled) {
		background-color: #dc2626; /* hover:bg-red-600 */
	}
	.btn-action-cancel:disabled {
		opacity: 0.6;
		cursor: not-allowed;
		/* background-color: #fca5a5; Let default browser style handle disabled bg */
	}
	.dark .btn-action-cancel {
		background-color: #dc2626; /* dark:bg-red-600 */
	}
	.dark .btn-action-cancel:hover:not(:disabled) {
		background-color: #b91c1c; /* dark:hover:bg-red-700 */
	}

	/* Checkbox style */
	.ui-checkbox {
		@apply w-4 h-4 text-blue-600 bg-gray-100 border-gray-300 rounded focus:ring-blue-500 dark:focus:ring-blue-600 dark:ring-offset-gray-800 focus:ring-2 dark:bg-gray-700 dark:border-gray-600;
	}
	.ui-radio {
		@apply w-4 h-4 text-blue-600 bg-gray-100 border-gray-300 focus:ring-blue-500 dark:focus:ring-blue-600 dark:ring-offset-gray-800 focus:ring-2 dark:bg-gray-700 dark:border-gray-600;
	}
	.ui-select {
		@apply block w-full pl-3 pr-10 py-2 text-sm border border-gray-300 focus:outline-none focus:ring-indigo-500 focus:border-indigo-500 rounded-md;
		background-color: white;
	}
	:global(.dark) .ui-select {
		background-color: #0d0d0d;
		border-color: #333333;
		color: white;
		color-scheme: dark;
	}

	.ui-input {
		@apply block w-full px-3 py-2 text-sm border border-gray-300 focus:outline-none focus:ring-indigo-500 focus:border-indigo-500 rounded-md;
		background-color: white;
	}
	:global(.dark) .ui-input {
		background-color: #0d0d0d;
		border-color: #333333;
		color: white;
		color-scheme: dark;
	}
	.btn-xs-secondary {
		@apply px-2 py-1 text-xs font-medium rounded border;
		@apply bg-gray-100 hover:bg-gray-200 text-gray-700 border-gray-300;
		@apply dark:bg-gray-700 dark:hover:bg-gray-600 dark:text-gray-200 dark:border-gray-500;
		@apply focus:outline-none focus:ring-2 focus:ring-indigo-500 focus:ring-offset-1 dark:focus:ring-offset-gray-800;
	}
</style>
