// src/lib/stores/layoutStore.js
import { writable, get } from 'svelte/store';
import { project } from '$lib/stores/projectStore.js'; // To get current project ID
import { transcriptStore } from '$lib/stores/transcriptStore.js'; // To get current media file path

const LOCAL_STORAGE_KEY_PREFIX = 'transcriptLayout_';
const DEFAULT_LAYOUT_KEY = 'Layout2'; // Default to 'Segment Block'

// Helper function to construct the specific local storage key
function getStorageKey(projectId, mediaPath) {
	if (!projectId || !mediaPath) return null;
	// Normalize mediaPath to create a consistent key, e.g., replace slashes
	const safeMediaPath = mediaPath.replace(/[\\/]/g, '_');
	return `${LOCAL_STORAGE_KEY_PREFIX}${projectId}_${safeMediaPath}`;
}

// Create the writable store
const { subscribe, set, update } = writable(DEFAULT_LAYOUT_KEY);

// Function to load layout from local storage
function loadLayoutForMedia(projectId, mediaPath) {
	if (typeof window === 'undefined') return DEFAULT_LAYOUT_KEY; // SSR guard

	const storageKey = getStorageKey(projectId, mediaPath);
	if (!storageKey) {
		set(DEFAULT_LAYOUT_KEY); // Fallback if key cannot be formed
		return DEFAULT_LAYOUT_KEY;
	}

	try {
		const storedLayout = localStorage.getItem(storageKey);
		if (storedLayout) {
			console.log(`[LayoutStore] Loaded layout '${storedLayout}' for ${mediaPath} in project ${projectId}`);
			set(storedLayout);
			return storedLayout;
		} else {
			console.log(`[LayoutStore] No layout stored for ${mediaPath} in project ${projectId}. Using default.`);
			set(DEFAULT_LAYOUT_KEY);
			return DEFAULT_LAYOUT_KEY;
		}
	} catch (error) {
		console.error('[LayoutStore] Error loading layout from localStorage:', error);
		set(DEFAULT_LAYOUT_KEY); // Fallback on error
		return DEFAULT_LAYOUT_KEY;
	}
}

// Function to save layout to local storage
function saveLayoutForMedia(projectId, mediaPath, layoutKey) {
	if (typeof window === 'undefined') return; // SSR guard

	const storageKey = getStorageKey(projectId, mediaPath);
	if (!storageKey) {
		console.warn('[LayoutStore] Could not save layout: Project ID or Media Path missing.');
		return;
	}

	try {
		localStorage.setItem(storageKey, layoutKey);
		console.log(`[LayoutStore] Saved layout '${layoutKey}' for ${mediaPath} in project ${projectId}`);
	} catch (error) {
		console.error('[LayoutStore] Error saving layout to localStorage:', error);
	}
}

// Custom store with methods
export const activeLayout = {
	subscribe,
	setLayout: (layoutKey) => {
		const currentProject = get(project);
		const currentTranscript = get(transcriptStore);

		if (currentProject && currentProject.id && currentTranscript && currentTranscript.selectedMediaFile?.path) {
			saveLayoutForMedia(currentProject.id, currentTranscript.selectedMediaFile.path, layoutKey);
			set(layoutKey);
		} else {
			// Fallback to just setting if project/media context is not fully available yet
			// This might happen during initial loads. The listeners below should correct it.
			console.warn('[LayoutStore] setLayout called without full project/media context. Setting locally only.');
			set(layoutKey);
		}
	},
	// Initialize with default or load from storage if project/media available
	init: () => {
		const currentProject = get(project);
		const currentTranscript = get(transcriptStore);
		if (currentProject && currentProject.id && currentTranscript && currentTranscript.selectedMediaFile?.path) {
			loadLayoutForMedia(currentProject.id, currentTranscript.selectedMediaFile.path);
		} else {
			set(DEFAULT_LAYOUT_KEY);
		}
	}
};

// --- Auto-load layout on project or media change ---

// Debounce helper
function debounce(func, wait) {
    let timeout;
    return function executedFunction(...args) {
        const later = () => {
            clearTimeout(timeout);
            func(...args);
        };
        clearTimeout(timeout);
        timeout = setTimeout(later, wait);
    };
}

const handleMediaOrProjectChange = debounce(() => {
    const currentProject = get(project);
    const currentTranscript = get(transcriptStore);

    // console.log('[LayoutStore] Media or Project changed. Project:', currentProject?.id, 'Media:', currentTranscript?.selectedMediaFile?.path);

    if (currentProject && currentProject.id && currentTranscript && currentTranscript.selectedMediaFile?.path) {
        // console.log(`[LayoutStore] Attempting to load layout for Project: ${currentProject.id}, Media: ${currentTranscript.selectedMediaFile.path}`);
        loadLayoutForMedia(currentProject.id, currentTranscript.selectedMediaFile.path);
    } else if (!currentTranscript?.selectedMediaFile?.path) {
        // If no media is selected (e.g., project just opened or media deselected), reset to default.
        // console.log('[LayoutStore] No media selected. Resetting layout to default.');
        set(DEFAULT_LAYOUT_KEY);
    }
    // If only project ID is missing, but media path is there, it's an odd state.
    // We could potentially try to load with a placeholder project ID or just wait.
    // For now, if project ID is missing, we don't load, relying on initial default or later valid state.
}, 100); // Debounce for 100ms to handle rapid changes

// Subscribe to project changes (specifically project ID)
project.subscribe(currentProjectState => {
    // console.log('[LayoutStore] Project store changed:', currentProjectState?.id);
	handleMediaOrProjectChange();
});

// Subscribe to transcriptStore changes (specifically selectedMediaFile.path)
transcriptStore.subscribe(currentTranscriptState => {
    // console.log('[LayoutStore] Transcript store changed. Selected Media Path:', currentTranscriptState?.selectedMediaFile?.path);
	handleMediaOrProjectChange();
});

// Initialize the store on load
if (typeof window !== 'undefined') {
    // Initial call to load if possible, but debounced handler will also run.
    // This helps if stores initialize in a slightly staggered way.
	activeLayout.init();
}

// Log store value changes for debugging (optional)
/*
activeLayout.subscribe(value => {
	console.log('[LayoutStore] Active layout changed to:', value);
});
*/
