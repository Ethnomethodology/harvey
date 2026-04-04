// src/lib/stores/layoutStore.js
import { writable } from 'svelte/store';

const LAYOUT_STORAGE_KEY = 'transcriptLayout'; // Key for local storage
const DEFAULT_LAYOUT_KEY = 'Layout2'; // Default layout

// --- Active Layout Store ---
function loadLayout() {
  if (typeof window === 'undefined') return DEFAULT_LAYOUT_KEY;
  try {
    const storedLayout = localStorage.getItem(LAYOUT_STORAGE_KEY);
    return storedLayout || DEFAULT_LAYOUT_KEY;
  } catch (error) {
    console.error('[LayoutStore] Error loading layout from localStorage:', error);
    return DEFAULT_LAYOUT_KEY;
  }
}

const { subscribe: layoutSubscribe, set: layoutSet } = writable(loadLayout());

function saveLayout(layoutKey) {
  if (typeof window === 'undefined') return;
  try {
    localStorage.setItem(LAYOUT_STORAGE_KEY, layoutKey);
  } catch (error) {
    console.error('[LayoutStore] Error saving layout to localStorage:', error);
  }
}

export const activeLayout = {
  subscribe: layoutSubscribe,
  setLayout: (layoutKey) => {
    saveLayout(layoutKey);
    layoutSet(layoutKey);
  }
};

layoutSubscribe(saveLayout);

// --- Left Panel Visibility Store ---
const PANEL_VISIBILITY_KEY = 'leftPanelVisible';
const DEFAULT_PANEL_VISIBLE = true;

function loadPanelVisibility() {
  if (typeof window === 'undefined') return DEFAULT_PANEL_VISIBLE;
  try {
    const storedValue = localStorage.getItem(PANEL_VISIBILITY_KEY);
    return storedValue !== null ? JSON.parse(storedValue) : DEFAULT_PANEL_VISIBLE;
  } catch (error) {
    console.error('[LayoutStore] Error loading panel visibility:', error);
    return DEFAULT_PANEL_VISIBLE;
  }
}

const {
  subscribe: visibilitySubscribe,
  set: visibilitySet,
  update: visibilityUpdate
} = writable(loadPanelVisibility());

function savePanelVisibility(isVisible) {
  if (typeof window === 'undefined') return;
  try {
    localStorage.setItem(PANEL_VISIBILITY_KEY, JSON.stringify(isVisible));
  } catch (error) {
    console.error('[LayoutStore] Error saving panel visibility:', error);
  }
}

export const leftPanelVisible = {
  subscribe: visibilitySubscribe,
  set: (value) => {
    savePanelVisibility(value);
    visibilitySet(value);
  },
  toggle: () => {
    visibilityUpdate((currentValue) => {
      const newValue = !currentValue;
      savePanelVisibility(newValue);
      return newValue;
    });
  }
};

visibilitySubscribe(savePanelVisibility);
