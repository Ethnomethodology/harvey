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
        'app-bg-dark': '#1e1e1e', // RGB(30, 30, 30)
        'app-text-inactive-dark': '#9a9a9a', // RGB(154, 154, 154)
        // --- ADDITION: Icon hover background color ---
        'app-icon-hover-dark': '#3c3c3c', // RGB(60, 60, 60)
        // --- END ADDITION ---
      },
      gridTemplateColumns: {
        '20': 'repeat(20, minmax(0, 1fr))'
      },
      spacing: {
        '1.5': '0.375rem', // 6px if 1rem = 16px
      }
    },
  },
  plugins: [
    require('@tailwindcss/typography'), // *** ADD THIS LINE ***
  ],
}