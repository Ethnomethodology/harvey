// src/lib/stores/refresherStore.js
import { writable } from 'svelte/store';

// A simple store that can be used to trigger updates in components.
// Update its value to trigger a refresh.
export const refresher = writable(0);

export function triggerRefresh() {
    refresher.update(n => n + 1);
}
