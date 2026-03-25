// src/lib/stores/panelStateStore.js
import { writable } from 'svelte/store';

const isBrowser = typeof window !== 'undefined';
const lsKey = 'harveyPanelState';

// Try to load persisted state
let loadedState = {};
if (isBrowser) {
    try {
        const stored = localStorage.getItem(lsKey);
        if (stored) loadedState = JSON.parse(stored);
    } catch (e) {
        console.warn("Failed to load panel state from localStorage:", e);
    }
}

const initialPanelState = {
    dataLeftPanelCollapsed: false, // For the main data/file list panel in DataView
    infoPanelCollapsed: false, // NEW - For the new combined InfoPanel, false = expanded by default
    activeInfoPanelTab: 'metadata', // NEW - Default active tab for the new InfoPanel/RightBar
    transcriptionPanelCollapsed: false, // For the main transcription settings/info panel in TranscriptionView
    tagsLeftPanelCollapsed: false, // Added
    groupDetailViewMode: loadedState.groupDetailViewMode || 'grid', // 'grid' or 'list', persisted
    // Add other panel states here as needed
};

const panelStateStore = writable(initialPanelState);

if (isBrowser) {
    panelStateStore.subscribe(state => {
        try {
            // Only persist specific keys like groupDetailViewMode to avoid unwanted UI state carrying over (like open modals/panels) unless desired
            const toPersist = {
                groupDetailViewMode: state.groupDetailViewMode
            };
            localStorage.setItem(lsKey, JSON.stringify(toPersist));
        } catch (e) {
            console.warn("Failed to save panel state to localStorage:", e);
        }
    });
}

// Function to toggle the Data Left Panel (main data list)
function toggleDataLeftPanel() {
    panelStateStore.update(state => ({
        ...state,
        dataLeftPanelCollapsed: !state.dataLeftPanelCollapsed
    }));
}

// NEW: Function to toggle the new InfoPanel
function toggleInfoPanel(collapsed) {
    panelStateStore.update(state => ({
        ...state,
        infoPanelCollapsed: collapsed ?? !state.infoPanelCollapsed
    }));
}

// NEW: Function to set the active tab for the InfoPanel/RightBar
function setActiveInfoPanelTab(tabName) {
    panelStateStore.update(state => ({
        ...state,
        activeInfoPanelTab: tabName
    }));
}

// Function to toggle the Transcription Panel (main settings/info in TranscriptionView)
function toggleTranscriptionPanel() {
    panelStateStore.update(state => ({
        ...state,
        transcriptionPanelCollapsed: !state.transcriptionPanelCollapsed
    }));
}

function toggleTagsLeftPanel() {
    panelStateStore.update(state => ({
        ...state,
        tagsLeftPanelCollapsed: !state.tagsLeftPanelCollapsed
    }));
}

function setGroupDetailViewMode(mode) {
    panelStateStore.update(state => ({
        ...state,
        groupDetailViewMode: mode
    }));
}

export default {
    subscribe: panelStateStore.subscribe,
    toggleDataLeftPanel,
    toggleInfoPanel,
    setActiveInfoPanelTab,
    // toggleLeftPanel, // REMOVED
    // toggleRightPanel, // REMOVED
    toggleTranscriptionPanel,
    toggleTagsLeftPanel,
    setGroupDetailViewMode,
    set: panelStateStore.set
};