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
        // New Dark Theme Palette (using CSS variables)
        'surface-1': 'var(--color-surface-1)',
        'surface-2': 'var(--color-surface-2)',
        'surface-3': 'var(--color-surface-3)',
        'border': 'var(--color-border)',

        'text-primary': 'var(--color-text-primary)',
        'text-secondary': 'var(--color-text-secondary)',
        'text-disabled': 'var(--color-text-disabled)',
        'text-accent': 'var(--color-text-accent)',

        'accent-primary': 'var(--color-accent-primary)',
        'accent-primary-hover': 'var(--color-accent-primary-hover)',
        'accent-background-hover': 'var(--color-accent-background-hover)',

        'status-success': 'var(--color-status-success)',
        'status-warning': 'var(--color-status-warning)',
        'status-error': 'var(--color-status-error)',

        // --- Mappings from old names for compatibility ---
        'dark-bg-primary': 'var(--color-surface-1)',
        'dark-bg-secondary': 'var(--color-surface-2)',
        'dark-bg-tertiary': 'var(--color-surface-3)',
        'dark-bg-form-field': 'var(--color-surface-2)',
        'dark-bg-icon-bar': 'var(--color-surface-3)',
        'dark-bg-lexical-editor': 'var(--color-surface-2)',

        'dark-text-primary': 'var(--color-text-primary)',
        'dark-text-secondary': 'var(--color-text-secondary)',
        'dark-text-tertiary': 'var(--color-text-disabled)',

        'dark-accent-primary': 'var(--color-accent-primary)',
        'dark-accent-secondary': 'var(--color-accent-primary-hover)',
        'dark-accent-text': 'var(--color-text-primary)',
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
