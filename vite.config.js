import { defineConfig } from "vite";
import { sveltekit } from "@sveltejs/kit/vite";

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

// https://vitejs.dev/config/
export default defineConfig(async () => ({
  plugins: [
    sveltekit()
  ],

  worker: {
    format: 'es'
  },

  build: {
    rollupOptions: {
      output: {
        manualChunks(id) {
          if (id.includes('src/lib/workers/waveformWorker.js')) {
            return 'waveformWorker';
          }
        }
      }
    }
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent vite from obscuring rust errors
  clearScreen: false,

  define: {
    '__BUILD_DATE__': JSON.stringify(new Date().toISOString()),
  },
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell vite to ignore watching `src-tauri`, `website`, and `.svelte-kit`
      ignored: ["**/src-tauri/**", "**/website/**", "**/.svelte-kit/**"],
    },
  },
  
}));
