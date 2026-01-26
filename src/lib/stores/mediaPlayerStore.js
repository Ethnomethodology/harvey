import { writable } from 'svelte/store';

function createMediaStore() {
    const { subscribe, set, update } = writable({
        src: null,
        currentTime: 0,
        duration: 0,
        paused: true,
        volume: 1,
        repeat: false,
        sourceComponent: null // 'minimal' | 'thin' | null
    });

    return {
        subscribe,
        set,
        update,
        updateState: (partial) => update(state => ({ ...state, ...partial })),
        reset: () => set({ src: null, currentTime: 0, duration: 0, paused: true, volume: 1, sourceComponent: null })
    };
}

export const mediaState = createMediaStore();

export const mediaCommands = writable(null); 
// Structure: { type: 'seek'|'play'|'pause'|'setSrc'|'togglePlay', value: any, timestamp: number, origin: string }

export function sendMediaCommand(type, value, origin) {
    mediaCommands.set({ type, value, timestamp: Date.now(), origin });
}