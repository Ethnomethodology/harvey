// src/lib/store/themeStore.js
import { writable } from 'svelte/store';
import { get } from 'svelte/store'; // To read store value inside functions
import { invoke } from '@tauri-apps/api/core'; // For backend integration

const THEME_STORAGE_KEY = 'harvey_theme_preference'; // Key for localStorage

// Possible values: 'light', 'dark', 'system'
// Read from localStorage first as a fallback or initial value before backend loads
const initialPreference = localStorage.getItem(THEME_STORAGE_KEY) || 'light';

export const themePreference = writable(initialPreference); // Stores 'light', 'dark', or 'system'
export const currentTheme = writable('light'); // Stores the resolved 'light' or 'dark'

/**
 * Applies the theme ('light' or 'dark') to the document root.
 * @param {'light' | 'dark'} resolvedTheme
 */
function applyTheme(resolvedTheme) {
  const root = document.documentElement;
  if (resolvedTheme === 'dark') {
    if (!root.classList.contains('dark')) {
      root.classList.add('dark');
      console.log('[ThemeStore] Applied dark theme');
    }
  } else {
    if (root.classList.contains('dark')) {
      root.classList.remove('dark');
      console.log('[ThemeStore] Applied light theme');
    }
  }
  // Update the derived store
  currentTheme.set(resolvedTheme);
}

/**
 * Checks the system's preferred color scheme.
 * @returns {'light' | 'dark'}
 */
function getSystemTheme() {
  if (window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches) {
    return 'dark';
  }
  return 'light';
}

/**
 * Resolves the theme preference ('system', 'light', 'dark') to a concrete theme ('light' or 'dark').
 * @param {'light' | 'dark' | 'system'} preference
 * @returns {'light' | 'dark'}
 */
function resolveTheme(preference) {
  if (preference === 'system') {
    return getSystemTheme();
  }
  return preference;
}

// Initial theme setup on load (using localStorage value initially)
applyTheme(resolveTheme(initialPreference));

// Subscribe to preference changes to save to localStorage and backend
themePreference.subscribe(async (preference) => {
  console.log(`[ThemeStore] Preference changed to: ${preference}`);
  // 1. Update localStorage immediately
  localStorage.setItem(THEME_STORAGE_KEY, preference);
  // 2. Apply the theme visually
  applyTheme(resolveTheme(preference));

  // --- 3. Backend Integration: Save Preference ---
  try {
    // Invoke the Rust command to save the preference
    await invoke('set_theme_preference', { theme: preference });
    console.log('[ThemeStore] Saved theme preference to backend.');
  } catch (error) {
    console.error('[ThemeStore] Failed to save theme preference to backend:', error);
    // Optional: Maybe show a non-blocking error to the user?
  }
  // -------------------------------------------
});

// Listen for system theme changes if preference is 'system'
if (window.matchMedia) {
  const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
  mediaQuery.addEventListener('change', () => {
    const preference = get(themePreference); // Get current preference
    if (preference === 'system') {
      console.log('[ThemeStore] System theme changed, reapplying.');
      applyTheme(getSystemTheme());
    }
  });
}

// Function to cycle theme preference: light -> dark -> system -> light ...
export function cycleThemePreference() {
  const currentPref = get(themePreference);
  let nextPref;
  if (currentPref === 'light') {
    nextPref = 'dark';
  } else if (currentPref === 'dark') {
    nextPref = 'system';
  } else {
    nextPref = 'light'; // Cycle back to light from system
  }
  themePreference.set(nextPref); // This triggers the subscription above
}

// --- Backend Integration: Load Preference ---
// Function to load preference from backend on startup (call this in +layout.svelte onMount)
export async function loadThemePreferenceFromBackend() {
  try {
    console.log('[ThemeStore] Attempting to load theme preference from backend...');
    // Invoke the Rust command
    const backendPreference = await invoke('get_theme_preference');

    // Check if a valid preference was returned ('light', 'dark', 'system')
    if (backendPreference && ['light', 'dark', 'system'].includes(backendPreference)) {
      console.log(`[ThemeStore] Loaded preference from backend: ${backendPreference}`);
      const currentPreference = get(themePreference);
      // Only update the store if the backend value is different from the current one
      // (which might have been loaded from localStorage)
      if (currentPreference !== backendPreference) {
        themePreference.set(backendPreference);
      }
      // Update localStorage as well to keep it synced with the backend (source of truth)
      localStorage.setItem(THEME_STORAGE_KEY, backendPreference);
    } else {
      // No need to set the store here, it already has the localStorage/default value
    }
  } catch (error) {
    console.error('[ThemeStore] Failed to load theme preference from backend:', error);
  }
}
// -----------------------------------------
