// src/lib/workers/waveformWorker.js
// Use @tauri-apps/api/fs for file reading in worker

let audioContext = null;
let currentAbortController = null; // To manage cancellation

// Utility function to generate audio peaks
function generateAudioPeaks(channelData, blockSize) {
  if (!channelData) return null;
  const length = channelData.length;
  const peaks = [];

  const numBlocks = Math.ceil(length / blockSize);
  const channelPeaks = new Float32Array(numBlocks * 2); // min, max per block

  for (let i = 0; i < numBlocks; i++) {
    const blockStart = i * blockSize;
    const blockEnd = Math.min(blockStart + blockSize, length);
    let min = 0.0;
    let max = 0.0;

    if (blockStart < blockEnd) {
      min = channelData[blockStart];
      max = channelData[blockStart];
      for (let j = blockStart + 1; j < blockEnd; j++) {
        const sample = channelData[j];
        if (sample < min) min = sample;
        if (sample > max) max = sample;
      }
    }
    channelPeaks[i * 2] = min;
    channelPeaks[i * 2 + 1] = max;
  }
  return channelPeaks;
}

self.onmessage = async (event) => {
  const { type, payload, id } = event.data;

  if (type === 'GENERATE_PEAKS') {
    const { channelData, sampleRate, filePath } = payload;

    // Abort any previous ongoing operation
    if (currentAbortController) {
      currentAbortController.abort();
    }
    currentAbortController = new AbortController();
    const signal = currentAbortController.signal;

    try {
      // Check for abortion before peak generation
      if (signal.aborted) {
        console.log(`[WaveformWorker] Peak generation aborted for ${filePath}`);
        return;
      }

      const peaksData = generateAudioPeaks(channelData, 512);

      self.postMessage({
        type: 'DECODE_AUDIO_COMPLETE',
        payload: {
          audioBuffer: null, // AudioBuffer is no longer transferred to the worker
          peaks: peaksData ? Array.from(peaksData) : null // Convert Float32Array to regular Array for transfer
        },
        id
      });
    } catch (error) {
      if (signal.aborted) {
        console.log(`[WaveformWorker] Operation cancelled for ${filePath}. Error:`, error.message);
      } else {
        console.error(`[WaveformWorker] Error generating peaks for ${filePath}:`, error);
        self.postMessage({
          type: 'DECODE_AUDIO_ERROR',
          payload: { error: error.message },
          id
        });
      }
    } finally {
      // Clear the current abort controller if this operation completed or errored out naturally
      if (currentAbortController === event.data.controller) {
        // Only clear if it's the one we set
        currentAbortController = null;
      }
    }
  } else if (type === 'CANCEL_CURRENT_OPERATION') {
    if (currentAbortController) {
      currentAbortController.abort();
      console.log('[WaveformWorker] Current operation cancelled by explicit message.');
      currentAbortController = null; // Clear it after aborting
    }
  }
};
