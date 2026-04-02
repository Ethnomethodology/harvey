<script>
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { Music } from '@lucide/svelte';

  export let file;

  let canvasRef;
  let isLoading = true;
  let error = false;

  // Fixed colors
  const TOP_COLOR = '#3b82f6'; // blue-500
  const BOTTOM_COLOR = '#60a5fa'; // blue-400

  function drawWaveform() {
    if (!file || !file.waveform_data || !canvasRef) {
      isLoading = false;
      return;
    }

    try {
      // waveform_data comes as a Uint8Array representing Float32 peaks (LE)
      // or sometimes as a regular array of floats if already processed.
      let rawPeaks;
      const data = file.waveform_data;

      // Robust type detection: if elements are small decimals, it's likely already floats
      const firstVal = data[0];
      const isFloatArray =
        !Number.isInteger(firstVal) && Math.abs(firstVal) < 2.0 && firstVal !== 0;

      if (isFloatArray) {
        rawPeaks = data instanceof Float32Array ? data : new Float32Array(data);
      } else {
        const uint8 = data instanceof Uint8Array ? data : new Uint8Array(data);
        if (uint8.byteOffset % 4 === 0) {
          rawPeaks = new Float32Array(uint8.buffer, uint8.byteOffset, uint8.byteLength / 4);
        } else {
          rawPeaks = new Float32Array(
            uint8.buffer.slice(uint8.byteOffset, uint8.byteOffset + uint8.byteLength)
          );
        }
      }

      if (!rawPeaks || rawPeaks.length === 0) {
        error = true;
        return;
      }

      // Implementation of "Partial Waveform" - show first 10 seconds
      const duration = file.duration_seconds || 60;
      const targetDuration = 10;

      if (duration > targetDuration) {
        const ratio = targetDuration / duration;
        const peaksToTake = Math.floor(rawPeaks.length * ratio);
        const finalCount = Math.max(peaksToTake + (peaksToTake % 2), 20);
        rawPeaks = rawPeaks.slice(0, finalCount);
      }

      const targetWidth = 100;
      const targetHeight = 100;

      const ctx = canvasRef.getContext('2d');
      const dpr = window.devicePixelRatio || 1;
      canvasRef.width = targetWidth * dpr;
      canvasRef.height = targetHeight * dpr;
      ctx.scale(dpr, dpr);

      canvasRef.style.width = '100%';
      canvasRef.style.height = '100%';

      const barCount = 40;
      const step = Math.max(1, Math.floor(rawPeaks.length / barCount));
      const displayPeaks = [];

      let maxObservedPeak = 0.01; // Avoid division by zero
      for (let i = 0; i < rawPeaks.length; i += step) {
        let maxVal = 0;
        for (let j = 0; j < step && i + j < rawPeaks.length; j++) {
          const val = Math.abs(rawPeaks[i + j]);
          if (val > maxVal) maxVal = val;
        }
        displayPeaks.push(maxVal);
        if (maxVal > maxObservedPeak) maxObservedPeak = maxVal;
      }

      // AUTO-NORMALIZATION: Scale quiet audio so the loudest peak fills 80% of height
      // We use a reasonable max of 1.0 but can go lower if the file is very quiet.
      const normalizationFactor = Math.min(1.0 / maxObservedPeak, 10.0); // Cap boost at 10x

      const barWidth = (targetWidth / displayPeaks.length) * 0.7;
      const gap = (targetWidth / displayPeaks.length) * 0.3;
      const centerY = targetHeight / 2;

      ctx.clearRect(0, 0, targetWidth, targetHeight);

      let visibleBars = 0;
      displayPeaks.forEach((peak, i) => {
        const normalized = isNaN(peak) ? 0 : Math.min(peak * normalizationFactor, 1.0);
        const barHeight = Math.max(normalized * (targetHeight * 0.8), 4);
        const x = i * (barWidth + gap);

        ctx.fillStyle = TOP_COLOR;
        const r = barWidth / 2;
        const y = centerY - barHeight / 2;
        const w = barWidth;
        const h = barHeight;

        ctx.beginPath();
        ctx.moveTo(x + r, y);
        ctx.lineTo(x + w - r, y);
        ctx.quadraticCurveTo(x + w, y, x + w, y + r);
        ctx.lineTo(x + w, y + h - r);
        ctx.quadraticCurveTo(x + w, y + h, x + w - r, y + h);
        ctx.lineTo(x + r, y + h);
        ctx.quadraticCurveTo(x, y + h, x, y + h - r);
        ctx.lineTo(x, y + r);
        ctx.quadraticCurveTo(x, y, x + r, y);
        ctx.fill();

        if (normalized > 0.05) visibleBars++;
      });

      console.debug(
        `[AudioThumbnail] ${file.name}: maxPeak=${maxObservedPeak.toFixed(4)}, boost=${normalizationFactor.toFixed(2)}x, visibleBars=${visibleBars}`
      );

      if (visibleBars === 0 && displayPeaks.length > 0) {
        error = true;
      } else {
        error = false;
      }
    } catch (e) {
      console.error('[AudioThumbnail] Failed to render waveform:', e);
      error = true;
    } finally {
      isLoading = false;
    }
  }

  // Reactive drawing triggered by file or canvasRef availability
  $: if (file && canvasRef) {
    drawWaveform();
  }

  onMount(() => {
    if (!file || !file.waveform_data) {
      isLoading = false;
      error = true;
    }
  });
</script>

<div class="w-full h-full relative overflow-hidden flex items-center justify-center">
  {#if error || !file.waveform_data}
    <div class="absolute inset-0 flex items-center justify-center text-gray-300 dark:text-gray-700">
      <Music size={40} strokeWidth={1} />
    </div>
  {:else}
    <!-- Always render the canvas if we have data so it can bind to canvasRef -->
    <canvas
      bind:this={canvasRef}
      class="w-full h-full object-contain transition-opacity duration-300 group-hover:opacity-100"
      class:opacity-0={isLoading}
      class:opacity-70={!isLoading}
    ></canvas>

    {#if isLoading}
      <div
        class="absolute inset-0 flex items-center justify-center bg-gray-50/50 dark:bg-gray-950/50"
      >
        <div
          class="w-5 h-5 border-2 border-blue-500/10 border-t-blue-500/50 rounded-full animate-spin"
        ></div>
      </div>
    {/if}
  {/if}
</div>
