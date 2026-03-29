<script>
    import { onMount } from 'svelte';
    import { invoke } from '@tauri-apps/api/core';
    import { Music } from 'lucide-svelte';

    export let file;

    let canvasRef;
    let isLoading = true;
    let error = false;

    // Fixed colors
    const TOP_COLOR = '#3b82f6'; // blue-500
    const BOTTOM_COLOR = '#60a5fa'; // blue-400

    onMount(async () => {
        if (!file || !file.waveform_data) {
            isLoading = false;
            error = true;
            return;
        }

        try {
            // waveform_data comes as an array of bytes from rust backend
            const rawPeaks = file.waveform_data;
            if (!rawPeaks || rawPeaks.length === 0) {
                error = true;
                return;
            }

            // Downsample peaks to fit the thumbnail width
            const targetWidth = 100;
            const targetHeight = 100;

            // Adjust canvas resolution for high DPI displays
            if (canvasRef) {
                const ctx = canvasRef.getContext('2d');
                const dpr = window.devicePixelRatio || 1;
                canvasRef.width = targetWidth * dpr;
                canvasRef.height = targetHeight * dpr;
                ctx.scale(dpr, dpr);
                canvasRef.style.width = `${targetWidth}%`;
                canvasRef.style.height = `${targetHeight}%`;

                // Calculate downsampled peaks
                const step = Math.ceil(rawPeaks.length / 50); // Show ~50 bars in the thumbnail
                const peaks = [];
                for (let i = 0; i < rawPeaks.length; i += step) {
                    let max = 0;
                    for (let j = 0; j < step && i + j < rawPeaks.length; j++) {
                        if (rawPeaks[i + j] > max) {
                            max = rawPeaks[i + j];
                        }
                    }
                    peaks.push(max);
                }

                // Render waveform
                const maxPeak = 127; // 8-bit unsigned int max
                const barWidth = (targetWidth / peaks.length) * 0.8;
                const gap = (targetWidth / peaks.length) * 0.2;

                const centerY = targetHeight / 2;

                ctx.clearRect(0, 0, targetWidth, targetHeight);
                ctx.lineCap = 'round';
                ctx.lineJoin = 'round';

                peaks.forEach((peak, i) => {
                    const normalized = peak / maxPeak;
                    // Ensure minimum height so very quiet audio still shows a line
                    const barHeight = Math.max(normalized * (targetHeight * 0.8), 2);
                    const x = i * (barWidth + gap);

                    // Top half
                    ctx.fillStyle = TOP_COLOR;
                    ctx.fillRect(x, centerY - barHeight / 2, barWidth, barHeight / 2);

                    // Bottom half
                    ctx.fillStyle = BOTTOM_COLOR;
                    ctx.fillRect(x, centerY, barWidth, barHeight / 2);
                });
            }
        } catch (e) {
            console.error('[AudioThumbnail] Failed to render waveform:', e);
            error = true;
        } finally {
            isLoading = false;
        }
    });
</script>

<div class="w-full h-full relative bg-gray-50 dark:bg-gray-950 overflow-hidden flex items-center justify-center">
    {#if isLoading}
        <div class="absolute inset-0 flex items-center justify-center">
            <div class="w-5 h-5 border-2 border-blue-500/10 border-t-blue-500/50 rounded-full animate-spin"></div>
        </div>
    {:else if error || !file.waveform_data}
        <div class="absolute inset-0 flex items-center justify-center text-gray-300 dark:text-gray-700">
            <Music size={40} strokeWidth={1} />
        </div>
    {:else}
        <canvas bind:this={canvasRef} class="w-full h-full object-contain opacity-70 transition-opacity duration-300 group-hover:opacity-100"></canvas>
    {/if}
</div>
