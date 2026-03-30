# Harvey UI Style Guide

This document outlines the design system for the Harvey desktop application. The goal is to create a clean, modern, accessible, and intuitive interface that feels professional and trustworthy.

## 1. Core Design Philosophy

- **Clean and Uncluttered**: Prioritize content and functionality. Use whitespace effectively to create a sense of calm and focus.
- **Modern Utility-First**: Harvey relies on **Tailwind CSS** for rapid, consistent styling directly within the Svelte template markup.
- **Component Driven**: We leverage **Flowbite-Svelte** to provide accessible, pre-built interactive components (Modals, Dropdowns, Toolbars, Inputs) that automatically hook into Tailwind's configuration.
- **Accessible**: Ensure all color combinations meet WCAG AA standards for contrast, and that typography is highly legible across both Light and Dark themes.

## 2. Tailwind CSS & Flowbite-Svelte

Harvey's styling is primarily driven by the `tailwind.config.js` file, which defines the core color palette, breakpoints, and plugins (like Flowbite).

**Avoid writing custom CSS rules in `<style>` blocks unless strictly necessary for highly specific, complex layouts (e.g., custom scrollbars, complex SVG overlays, or deeply nested Tabulator grids).** Instead, compose utility classes.

### Theming: Light & Dark Mode

Harvey supports full dark mode via the `class` strategy in Tailwind. All UI components must explicitly define both states using the `dark:` variant prefix.

*   **Example (Backgrounds & Text):**
    ```html
    <!-- Correct -->
    <div class="bg-white dark:bg-gray-900 text-gray-900 dark:text-gray-100">
        Content
    </div>
    ```

*   **Example (Borders):**
    ```html
    <!-- Correct -->
    <div class="border border-gray-200 dark:border-gray-700">
        Divided Section
    </div>
    ```

The active theme is managed globally by `$themePreference` (in `src/lib/stores/themeStore.js`) and toggled via the Application Configuration menu.

## 3. Color Palettes

We utilize Tailwind's extended color palette, heavily relying on the `gray` scale for structure and `blue` for primary brand accents and interactive states.

### Light Mode Focus
*   **Backgrounds**: `bg-gray-50` for application canvases, `bg-white` for surface cards.
*   **Text**: `text-gray-900` for primary content, `text-gray-500` for secondary metadata.
*   **Borders**: `border-gray-200` or `border-gray-300`.
*   **Interactive**: `bg-blue-600` for primary buttons, `text-blue-600` for text links.

### Dark Mode Focus
*   **Backgrounds**: `dark:bg-gray-950` for application canvases, `dark:bg-gray-900` or `dark:bg-gray-800` for surface cards.
*   **Text**: `dark:text-white` or `dark:text-gray-100` for primary content, `dark:text-gray-400` for secondary metadata.
*   **Borders**: `dark:border-gray-700` or `dark:border-gray-600`.
*   **Interactive**: `dark:bg-blue-700` for primary buttons, `dark:text-blue-500` for text links.

## 4. Typography

The UI will use **Inter**, a modern, professional, and highly legible sans-serif font.

- **Font Family**: `font-sans` (mapped to Inter in Tailwind config).
- **Scale**: Use standard Tailwind text utilities (`text-xs`, `text-sm`, `text-base`, `text-lg`, `text-xl`, `text-2xl`).

## 5. Spacing and Layout

The layout is built on Tailwind's standard 4px rem-based grid system.

*   **Padding/Margins**: Use multiples of this base unit (`p-4`, `m-2`, `gap-3`).
*   **Flexbox/Grid**: Rely heavily on `flex`, `flex-col`, `items-center`, `justify-between`, and CSS Grid (`grid-cols-2`) for structural alignment over custom floats or absolute positioning (unless building floating UI like toolbars).
*   **Border Radius**: A standard `rounded-md` (6px) or `rounded-lg` (8px) is used for most components like buttons, inputs, and cards.

## 6. Flowbite Component Standards

When building interactive UI, prefer Flowbite-Svelte components over raw HTML tags to ensure accessibility and consistent styling.

### Buttons (`<Button>`)
*   **Primary**: `color="blue"` (Auto-maps to solid blue background).
*   **Secondary/Outline**: `color="alternative"` (Auto-maps to bordered, transparent background).
*   **Destructive**: `color="red"` (Used for delete confirmations).
*   **Icon Buttons**: Use standard HTML `<button>` tags with Tailwind utilities for precise sizing and hover states (e.g., `p-1.5 rounded-md hover:bg-gray-100 dark:hover:bg-gray-800`) when wrapping Lucide icons.

### Inputs (`<Input>`, `<Select>`, `<Textarea>`)
*   Always bind values using Svelte's `bind:value`.
*   Ensure text inputs, specifically those dealing with tags, comments, or technical configurations, disable native browser interference to prevent frustrating autocorrect behavior:
    ```html
    <Input type="text" autocomplete="off" autocorrect="off" spellcheck="false" />
    ```

### Icons
*   **Set**: **Lucide Svelte** (`@lucide/svelte`).
*   **Usage**: Import directly and apply Tailwind classes for sizing and color:
    ```svelte
    <script>
        import { Settings } from '@lucide/svelte';
    </script>
    <Settings class="w-4 h-4 text-gray-500 dark:text-gray-400" />
    ```
