// src/lib/stores/panelStateStore.svelte.js

const isBrowser = typeof window !== 'undefined';
const lsKey = 'harveyPanelState';

// Try to load persisted state
let loadedState = {};
if (isBrowser) {
  try {
    const stored = localStorage.getItem(lsKey);
    if (stored) loadedState = JSON.parse(stored);
  } catch (e) {
    console.warn('Failed to load panel state from localStorage:', e);
  }
}

// Svelte 5 Rune for panel state
export const panelState = $state({
  dataLeftPanelCollapsed: false, // For the main data/file list panel in DataView
  infoPanelCollapsed: false, // For the new combined InfoPanel, false = expanded by default
  activeInfoPanelTab: 'metadata', // Default active tab for the new InfoPanel/RightBar
  transcriptionPanelCollapsed: false, // For the main transcription settings/info panel in TranscriptionView
  tagsLeftPanelCollapsed: false,
  groupDetailViewMode: loadedState.groupDetailViewMode || 'list'
});

// Svelte 5 Effect for persistence
if (isBrowser) {
  $effect.root(() => {
    $effect(() => {
      try {
        const toPersist = {
          groupDetailViewMode: panelState.groupDetailViewMode
        };
        localStorage.setItem(lsKey, JSON.stringify(toPersist));
      } catch (e) {
        console.warn('Failed to save panel state to localStorage:', e);
      }
    });
  });
}

// State mutations
export function toggleDataLeftPanel() {
  panelState.dataLeftPanelCollapsed = !panelState.dataLeftPanelCollapsed;
}

export function toggleInfoPanel(collapsed) {
  panelState.infoPanelCollapsed = collapsed ?? !panelState.infoPanelCollapsed;
}

export function setActiveInfoPanelTab(tabName) {
  panelState.activeInfoPanelTab = tabName;
}

export function toggleTranscriptionPanel() {
  panelState.transcriptionPanelCollapsed = !panelState.transcriptionPanelCollapsed;
}

export function toggleTagsLeftPanel() {
  panelState.tagsLeftPanelCollapsed = !panelState.tagsLeftPanelCollapsed;
}

export function setGroupDetailViewMode(mode) {
  panelState.groupDetailViewMode = mode;
}

// For backward compatibility during migration, we could export a store-like object,
// but the recommendation is to use the state directly.
// We rename the export to 'panelState' but also export it as 'panelStateStore' if needed.
const panelStateStore = {
  get dataLeftPanelCollapsed() { return panelState.dataLeftPanelCollapsed; },
  get infoPanelCollapsed() { return panelState.infoPanelCollapsed; },
  get activeInfoPanelTab() { return panelState.activeInfoPanelTab; },
  get transcriptionPanelCollapsed() { return panelState.transcriptionPanelCollapsed; },
  get tagsLeftPanelCollapsed() { return panelState.tagsLeftPanelCollapsed; },
  get groupDetailViewMode() { return panelState.groupDetailViewMode; },
  toggleDataLeftPanel,
  toggleInfoPanel,
  setActiveInfoPanelTab,
  toggleTranscriptionPanel,
  toggleTagsLeftPanel,
  setGroupDetailViewMode
};

export default panelStateStore;
