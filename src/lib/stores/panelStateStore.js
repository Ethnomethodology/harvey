// src/lib/stores/panelStateStore.js
import { writable } from 'svelte/store';

const initialPanelState = {
    notesLeftPanelCollapsed: false, // For the main data/file list panel in NotesView
    // leftCollapsed: false, // REMOVED - old state for LeftInfoPanel in specific views
    // rightCollapsed: true, // REMOVED - old state for RightInfoPanel in specific views
    infoPanelCollapsed: false, // NEW - For the new combined InfoPanel, false = expanded by default
    activeInfoPanelTab: 'metadata', // NEW - Default active tab for the new InfoPanel/RightBar
    transcriptionPanelCollapsed: false, // For the main transcription settings/info panel in TranscriptionsView
    // Add other panel states here as needed
};

const panelStateStore = writable(initialPanelState);

// Function to toggle the Notes Left Panel (main data list)
function toggleNotesLeftPanel() {
    panelStateStore.update(state => ({
        ...state,
        notesLeftPanelCollapsed: !state.notesLeftPanelCollapsed
    }));
}

// NEW: Function to toggle the new InfoPanel
function toggleInfoPanel() {
    panelStateStore.update(state => ({
        ...state,
        infoPanelCollapsed: !state.infoPanelCollapsed
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

export default {
    subscribe: panelStateStore.subscribe,
    toggleNotesLeftPanel,
    toggleInfoPanel,
    setActiveInfoPanelTab,
    // toggleLeftPanel, // REMOVED
    // toggleRightPanel, // REMOVED
    toggleTranscriptionPanel,
    set: panelStateStore.set
};
