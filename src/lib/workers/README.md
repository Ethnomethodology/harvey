# Web Workers (`src/lib/workers`)

**Purpose:** Offloads heavy, blocking computations—such as audio decoding, waveform peak generation, and complex string matching—from the main browser UI thread to background threads, ensuring a smooth and responsive user experience.

## Exported Workers

### `waveformWorker.js`

Handles the generation of audio peaks (min/max amplitude values) used by the `InteractiveWaveform` and `VerticalWaveform` components to render visualizations.

#### Input Messages (Main Thread -> Worker)

- **`GENERATE_PEAKS`**: `{ channelData: Float32Array, sampleRate: number, filePath: string }`

#### Output Messages (Worker -> Main Thread)

- **`DECODE_AUDIO_COMPLETE`**: `{ peaks: Array<number> }`
- **`DECODE_AUDIO_ERROR`**: `{ error: string }`

### `pdfAnnotationMatcher.worker.js`

Handles heavy computational matching of text annotations against extracted PDF document text, likely to find exact text coordinates or fuzzy matches.

#### Processing Logic

Workers receive a `postMessage` payload, perform their intensive `for` loops or array manipulations, and return the simplified/reduced dataset (like a small array of peaks or a list of matched bounding boxes) via `postMessage` back to the invoking Svelte component or service.
