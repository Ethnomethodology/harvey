// src/lib/stores/panelStateStore.js
import { writable } from 'svelte/store';

const initialPanelState = {
    leftCollapsed: false,
    rightCollapsed: false,
};

const panelState = writable(initialPanelState);

function toggleLeftPanel() {
    panelState.update(state => ({ ...state, leftCollapsed: !state.leftCollapsed }));
}

function toggleRightPanel() {
    panelState.update(state => ({ ...state, rightCollapsed: !state.rightCollapsed }));
}

export default {
    subscribe: panelState.subscribe,
    toggleLeftPanel,
    toggleRightPanel
};
