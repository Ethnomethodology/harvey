// src/lib/stores/layoutViewStore.js
import { writable } from 'svelte/store';

const STORE_KEY = 'leftPanelVisible';

// Function to get initial value from localStorage or default to true
const getInitialVisibility = () => {
    if (typeof localStorage !== 'undefined') {
        const storedValue = localStorage.getItem(STORE_KEY);
        if (storedValue !== null) {
            return JSON.parse(storedValue);
        }
    }
    return true; // Default to visible
};

// Create a writable store with the initial value
const { subscribe, set, update } = writable(getInitialVisibility());

// Function to update localStorage whenever the store value changes
subscribe(value => {
    if (typeof localStorage !== 'undefined') {
        localStorage.setItem(STORE_KEY, JSON.stringify(value));
    }
});

export const leftPanelVisible = {
    subscribe,
    set,
    toggle: () => update(visible => !visible)
};
