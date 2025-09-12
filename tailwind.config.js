// tailwind.config.js
/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./src/**/*.{html,js,svelte,ts}"
  ],
  darkMode: 'class',
  theme: {
    extend: {
      colors: {
        // New Dark Theme Palette
        'dark-bg-primary': '#0d0d0d',    // For main layout bars (top, bottom, side)
        'dark-bg-secondary': '#1a1a1a',  // For panels, cards, and other content areas
        'dark-bg-tertiary': '#2a2a2a',   // For hovered items, borders, and separators

        'dark-text-primary': '#e6e6e6',      // Main text color (slightly off-white)
        'dark-text-secondary': '#9a9a9a',  // Lighter text for inactive/secondary info
        'dark-text-tertiary': '#6b7280',   // Even lighter text for disabled/tertiary info

        'dark-accent-primary': '#3B82F6', // A vibrant blue for interactive elements
        'dark-accent-secondary': '#2563EB', // A slightly darker blue for hover states on accents
        'dark-accent-text': '#EFF6FF',     // Text color for on-accent elements
      },
      gridTemplateColumns: {
        '20': 'repeat(20, minmax(0, 1fr))'
      }
    },
  },
  plugins: [
    require('@tailwindcss/typography'), // *** ADD THIS LINE ***
  ],
}