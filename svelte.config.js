// harvey/svelte.config.js
import adapter from '@sveltejs/adapter-static';
import { mdsvex } from 'mdsvex';
import { sveltePreprocess } from 'svelte-preprocess';
import path from 'path'; // Import the path module

/** @type {import('@sveltejs/kit').Config} */
const config = {
  extensions: ['.svelte', '.md'],
  preprocess: [
    sveltePreprocess({ postcss: true }),
    mdsvex({
      extensions: ['.md'],
    })
  ],
  kit: {
    // Use static adapter to generate a static build into `build/`
    adapter: adapter({
      pages: 'build',
      assets: 'build',
      fallback: null
    }),
    alias: {
      $lib: path.resolve('./src/lib') // This sets $lib to src/lib
    },
    // Pre-render all routes by default
    prerender: {
      entries: ['*']
    }
  }
};

export default config;
