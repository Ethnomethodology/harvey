import { writable } from 'svelte/store';
import { browser } from '$app/environment';

export const isMediaEditorOpen = writable(false);

const storedEditMode = browser ? localStorage.getItem('isLexicalEditMode') : null;
export const isLexicalEditMode = writable(storedEditMode === null ? true : storedEditMode === 'true');

if (browser) {
	isLexicalEditMode.subscribe((value) => {
		localStorage.setItem('isLexicalEditMode', String(value));
	});
}
