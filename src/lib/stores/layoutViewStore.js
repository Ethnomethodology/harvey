// src/lib/stores/layoutViewStore.js
import { writable } from 'svelte/store';

const LOCAL_STORAGE_KEY = 'transcriptLayout'; // Key for local storage
const DEFAULT_LAYOUT_KEY = 'Layout1'; // Default layout

// Load layout from local storage
function loadLayout() {
	if (typeof window === 'undefined') return DEFAULT_LAYOUT_KEY;

	try {
		const storedLayout = localStorage.getItem(LOCAL_STORAGE_KEY);
		return storedLayout || DEFAULT_LAYOUT_KEY;
	} catch (error) {
		console.error('[LayoutViewStore] Error loading layout from localStorage:', error);
		return DEFAULT_LAYOUT_KEY;
	}
}

const { subscribe, set, update } = writable(loadLayout());

// Save layout to local storage
function saveLayout(layoutKey) {
	if (typeof window === 'undefined') return;

	try {
		localStorage.setItem(LOCAL_STORAGE_KEY, layoutKey);
	} catch (error) {
		console.error('[LayoutViewStore] Error saving layout to localStorage:', error);
	}
}

export const activeLayout = {
	subscribe,
	setLayout: (layoutKey) => {
		saveLayout(layoutKey);
		set(layoutKey);
	},
};

subscribe(saveLayout);
