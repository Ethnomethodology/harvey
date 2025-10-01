import { writable } from 'svelte/store';

const STORAGE_KEY = 'harvey-waveform-layout';
const initialWaveformLayout = 'horizontal'; // Default to horizontal

const createWaveformLayoutStore = () => {
  const storedLayout = typeof window !== 'undefined' ? localStorage.getItem(STORAGE_KEY) : null;
  const initialLayout = storedLayout && ['horizontal', 'vertical', 'none'].includes(storedLayout) ? storedLayout : initialWaveformLayout;

  const { subscribe, set, update } = writable(initialLayout);

  return {
    subscribe,
    setLayout: (layout) => {
      if (['horizontal', 'vertical', 'none'].includes(layout)) {
        if (typeof window !== 'undefined') {
          localStorage.setItem(STORAGE_KEY, layout);
        }
        set(layout);
      } else {
        console.warn(`[WaveformLayoutStore] Invalid layout type: ${layout}. Setting to default.`);
        if (typeof window !== 'undefined') {
          localStorage.setItem(STORAGE_KEY, initialWaveformLayout);
        }
        set(initialWaveformLayout);
      }
    },
    reset: () => {
      if (typeof window !== 'undefined') {
        localStorage.setItem(STORAGE_KEY, initialWaveformLayout);
      }
      set(initialWaveformLayout);
    }
  };
};

const waveformLayoutStore = createWaveformLayoutStore();

export default waveformLayoutStore;