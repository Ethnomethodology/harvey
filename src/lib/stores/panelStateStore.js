// src/lib/stores/panelStateStore.js
import { writable } from 'svelte/store';

const initialPanelState = {
    leftCollapsed: false,
    rightCollapsed: false,
    notesLeftPanelCollapsed: false,
};

const panelState = writable(initialPanelState);

function toggleLeftPanel() {
    panelState.update(state => ({ ...state, leftCollapsed: !state.leftCollapsed }));
}

function toggleRightPanel() {
    panelState.update(state => ({ ...state, rightCollapsed: !state.rightCollapsed }));
}

function toggleNotesLeftPanel() {
    panelState.update(state => ({ ...state, notesLeftPanelCollapsed: !state.notesLeftPanelCollapsed }));
}

export default {
    subscribe: panelState.subscribe,
    toggleLeftPanel,
    toggleRightPanel,
    toggleNotesLeftPanel
};
