// src/lib/stores/panelStateStore.js
import { writable } from 'svelte/store';

const initialPanelState = {
    dataLeftPanelCollapsed: false, // For the main data/file list panel in DataView
    // leftCollapsed: false, // REMOVED - old state for LeftInfoPanel in specific views
    // rightCollapsed: true, // REMOVED - old state for RightInfoPanel in specific views
    infoPanelCollapsed: false, // NEW - For the new combined InfoPanel, false = expanded by default
    activeInfoPanelTab: 'metadata', // NEW - Default active tab for the new InfoPanel/RightBar
    transcriptionPanelCollapsed: false, // For the main transcription settings/info panel in TranscriptionsView
    tagsLeftPanelCollapsed: false, // Added
    // Add other panel states here as needed
};

const panelStateStore = writable(initialPanelState);

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

// Function to toggle the Transcription Panel (main settings/info in TranscriptionsView)
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

export default {
    subscribe: panelStateStore.subscribe,
    toggleDataLeftPanel,
    toggleInfoPanel,
    setActiveInfoPanelTab,
    // toggleLeftPanel, // REMOVED
    // toggleRightPanel, // REMOVED
    toggleTranscriptionPanel,
    toggleTagsLeftPanel,
    set: panelStateStore.set
};