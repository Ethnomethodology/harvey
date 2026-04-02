import { writable } from 'svelte/store';

export const searchQuery = writable('');
export const showSearchBox = writable(false);
