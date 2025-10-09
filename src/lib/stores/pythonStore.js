import { writable } from 'svelte/store';

// This store will hold the status of whether the required Python libraries are installed.
export const arePythonLibsInstalled = writable(false);
