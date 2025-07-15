// src/lib/stores/layoutStore.js
import { writable } from 'svelte/store';

const LOCAL_STORAGE_KEY = 'transcriptLayout'; // Global key
const DEFAULT_LAYOUT_KEY = 'Layout1'; // Default to 'Detailed Table'

// Function to load layout from local storage
function loadLayout() {
	if (typeof window === 'undefined') return DEFAULT_LAYOUT_KEY; // SSR guard

	try {
		const storedLayout = localStorage.getItem(LOCAL_STORAGE_KEY);
		if (storedLayout) {
			console.log(`[LayoutStore] Loaded global layout '${storedLayout}'`);
			return storedLayout;
		} else {
			console.log(`[LayoutStore] No global layout stored. Using default.`);
			return DEFAULT_LAYOUT_KEY;
		}
	} catch (error) {
		console.error('[LayoutStore] Error loading layout from localStorage:', error);
		return DEFAULT_LAYOUT_KEY; // Fallback on error
	}
}

// Create the writable store with the initial value loaded from local storage
const { subscribe, set, update } = writable(loadLayout());

// Function to save layout to local storage
function saveLayout(layoutKey) {
	if (typeof window === 'undefined') return; // SSR guard

	try {
		localStorage.setItem(LOCAL_STORAGE_KEY, layoutKey);
		console.log(`[LayoutStore] Saved global layout '${layoutKey}'`);
	} catch (error) {
		console.error('[LayoutStore] Error saving layout to localStorage:', error);
	}
}

// Custom store with methods
export const activeLayout = {
	subscribe,
	setLayout: (layoutKey) => {
		saveLayout(layoutKey);
		set(layoutKey);
	},
	// No explicit init needed as the store is initialized with loadLayout()
	// and setLayout handles saving on change.
};

// Subscribe to store changes to persist the value to local storage
subscribe(value => {
    saveLayout(value);
});

// Log store value changes for debugging (optional)
/*
activeLayout.subscribe(value => {
	console.log('[LayoutStore] Active layout changed to:', value);
});
*/
