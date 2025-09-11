/** @type {import('tailwindcss').Config} */
export default {
  content: ["./src/**/*.{html,js,svelte,ts}"],
  darkMode: 'class',
  theme: {
    extend: {
      colors: {
        primary: {
          DEFAULT: 'rgb(var(--color-primary) / <alpha-value>)',
          hover: 'rgb(var(--color-primary-hover) / <alpha-value>)',
          active: 'rgb(var(--color-primary-active) / <alpha-value>)',
        },
        'bg-primary': 'rgb(var(--color-bg-primary) / <alpha-value>)',
        'bg-secondary': 'rgb(var(--color-bg-secondary) / <alpha-value>)',
        'text-primary': 'rgb(var(--color-text-primary) / <alpha-value>)',
        'text-secondary': 'rgb(var(--color-text-secondary) / <alpha-value>)',
        border: 'rgb(var(--color-border) / <alpha-value>)',
        success: 'rgb(var(--color-success) / <alpha-value>)',
        warning: 'rgb(var(--color-warning) / <alpha-value>)',
        error: 'rgb(var(--color-error) / <alpha-value>)',
      },
      fontFamily: {
        sans: ['"Inter"', 'sans-serif'],
        mono: ['"JetBrains Mono"', 'monospace'],
      },
      fontSize: {
        'caption': ['0.75rem', '1rem'],      // 12px
        'label': ['0.875rem', '1.25rem'],   // 14px
        'base': ['1rem', '1.5rem'],         // 16px
        'h3': ['1.125rem', '1.75rem'],      // 18px
        'h2': ['1.5rem', '2rem'],           // 24px
        'h1': ['2rem', '2.5rem'],           // 32px
      },
      borderRadius: {
        'md': '6px',
      },
      gridTemplateColumns: {
        '20': 'repeat(20, minmax(0, 1fr))'
      }
    },
  },
  plugins: [
    require('@tailwindcss/typography'),
  ],
};