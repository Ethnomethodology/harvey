// svelte.config.js
import adapter from '@sveltejs/adapter-auto';
import { sveltePreprocess } from 'svelte-preprocess';
import path from 'path';  // Import the path module

/** @type {import('@sveltejs/kit').Config} */
const config = {
  preprocess: sveltePreprocess({
    postcss: true,
  }),
  kit: {
    adapter: adapter(),
    alias: {
      $lib: path.resolve('./src/lib') // This sets $lib to src/lib
    }
    // other options...
  }
};

export default config;
