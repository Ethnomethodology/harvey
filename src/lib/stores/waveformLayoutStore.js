import { writable } from 'svelte/store';

const initialWaveformLayout = 'horizontal'; // Default to horizontal

const createWaveformLayoutStore = () => {
  const { subscribe, set, update } = writable(initialWaveformLayout);

  return {
    subscribe,
    setLayout: (layout) => {
      if (['horizontal', 'vertical', 'none'].includes(layout)) {
        set(layout);
      } else {
        console.warn(`[WaveformLayoutStore] Invalid layout type: ${layout}. Setting to default.`);
        set(initialWaveformLayout);
      }
    },
    reset: () => set(initialWaveformLayout)
  };
};

const waveformLayoutStore = createWaveformLayoutStore();

export default waveformLayoutStore;
