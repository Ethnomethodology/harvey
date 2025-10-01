# Harvey UI Style Guide

This document outlines the design system for the Harvey desktop application. The goal is to create a clean, modern, accessible, and intuitive interface that feels professional and trustworthy.

## 1. Core Design Philosophy

- **Clean and Uncluttered**: Prioritize content and functionality. Use whitespace effectively to create a sense of calm and focus.
- **Modern**: Employ current best practices without being overly trendy. The design should feel timeless and durable.
- **Accessible**: Ensure all color combinations meet WCAG AA standards for contrast, and that typography is highly legible.
- **Intuitive**: The design should guide the user. Visual hierarchy, interactive cues, and consistent patterns are paramount.
- **Unique**: The design should feel bespoke to Harvey, not like a generic template.

## 2. Color Palettes

Colors are defined as CSS custom properties for easy theming. The primary brand color is a professional and academic **Slate Blue**.

### Primary Color: Slate

| Role          | Light Mode | Dark Mode  | HEX (Light) | HEX (Dark) |
|---------------|------------|------------|-------------|------------|
| Primary       | `primary`  | `primary`  | `#4A6987`   | `#6B97C2`  |
| Lighter (Hover) | `primary-hover` | `primary-hover` | `#6B8AAA` | `#8CB5D9`  |
| Darker (Active) | `primary-active`| `primary-active`| `#3A536C` | `#4A6987`  |

### Light Mode Palette

| Role              | Name            | HEX       | Description                                      |
|-------------------|-----------------|-----------|--------------------------------------------------|
| Background        | `bg-primary`    | `#F8F9FA` | A very light, neutral off-white.                 |
| Surface           | `bg-secondary`  | `#FFFFFF` | For cards, sidebars, and modals.                 |
| Border            | `border`        | `#E9ECEF` | Subtle, low-contrast for defining edges.         |
| Primary Text      | `text-primary`  | `#212529` | A very dark gray for maximum readability.        |
| Secondary Text    | `text-secondary`| `#6C757D` | For labels, hints, and less important info.      |
| Interactive       | `accent`        | `#4A6987` | The primary brand color for interactive elements.|
| Success           | `success`       | `#2F9E44` | Green for success states.                        |
| Warning           | `warning`       | `#F7B42C` | Amber for warnings.                              |
| Error             | `error`         | `#D63939` | Red for errors and destructive actions.          |

### Dark Mode Palette

| Role              | Name            | HEX       | Description                                      |
|-------------------|-----------------|-----------|--------------------------------------------------|
| Background        | `bg-primary`    | `#1A1D21` | A very dark, desaturated blue-gray.              |
| Surface           | `bg-secondary`  | `#2C3138` | A slightly lighter dark shade for cards.         |
| Border            | `border`        | `#373C44` | Subtle, just visible against the surface.        |
| Primary Text      | `text-primary`  | `#F8F9FA` | A very light gray for maximum readability.       |
| Secondary Text    | `text-secondary`| `#ADB5BD` | Lighter gray for labels, maintaining contrast.   |
| Interactive       | `accent`        | `#6B97C2` | A brighter slate blue to stand out.              |
| Success           | `success`       | `#40C057` | A brighter green for dark backgrounds.           |
| Warning           | `warning`       | `#FCC419` | A brighter amber for dark backgrounds.           |
| Error             | `error`         | `#F06565` | A brighter red for dark backgrounds.             |

---

## 3. Typography

### Font Family

The UI will use **Inter**, a modern, professional, and highly legible sans-serif font. It is well-suited for user interfaces and provides excellent readability across a wide range of sizes and weights. It should be imported from a font service like Google Fonts.

- **Font Family**: `Inter, sans-serif`

### Typographic Scale

A consistent, rhythmic scale is used for all text elements.

| Element         | Font Size (rem) | Font Size (px) | Font Weight |
|-----------------|-----------------|----------------|-------------|
| H1 / Page Title | `2rem`          | 32px           | `600` (Semi-bold) |
| H2 / Section Title| `1.5rem`        | 24px           | `600` (Semi-bold) |
| H3 / Card Title | `1.125rem`      | 18px           | `600` (Semi-bold) |
| Body            | `1rem`          | 16px           | `400` (Regular)   |
| Label           | `0.875rem`      | 14px           | `500` (Medium)    |
| Caption         | `0.75rem`       | 12px           | `400` (Regular)   |

---

## 4. Spacing and Layout

### Base Unit

The layout is built on a **4px** grid system. All margins, padding, and gaps should use multiples of this base unit (4, 8, 12, 16, 24, 32, etc.).

- **Base Spacing Unit**: `1` unit = `0.25rem` (4px).

### Borders & Radius

- **Border Style**: `1px solid var(--color-border)`
- **Border Radius**: A standard `6px` (`rounded-md` in Tailwind) is used for most components like buttons, inputs, and cards to maintain a consistent, soft-modern look.

---

## 5. Component Styles

### Buttons

- **Primary**: Solid fill (`bg-accent`, `text-white`). Hover: `bg-primary-hover`.
- **Secondary**: Outline (`border-border`, `text-accent`). Hover: `bg-accent/10`.
- **Tertiary (Ghost)**: No border or background (`text-accent`). Hover: `bg-accent/10`.
- **States**: `focus` state should have a visible ring. `disabled` state should use `opacity-50` and `cursor-not-allowed`.

### Input Fields (Text, Dropdowns)

- **Default**: `bg-secondary`, `text-primary`, `border-border`.
- **Focus**: Border color changes to `border-accent`. A subtle focus ring should be visible.
- **Disabled**: `bg-primary`, `text-secondary`, `opacity-50`.

### Tabs

- **Active**: `text-accent`, with a solid `border-accent` underline.
- **Inactive**: `text-secondary`, no border. Hover: `text-primary`.

### Cards / Containers

- **Style**: `bg-secondary` with `border-border` and a `rounded-md` radius.
- **Shadow**: A very subtle box shadow to create depth: `0 1px 3px rgba(0, 0, 0, 0.02), 0 1px 2px rgba(0, 0, 0, 0.04)`.

### Icons

- **Set**: **Lucide** (already in the project). It's clean, simple, and comprehensive.
- **Color**: Icons should use `text-secondary` by default. When part of an interactive element (like a button), they can adopt the state color (e.g., `text-accent` on hover).

---

## 6. CSS Custom Properties Implementation

The following CSS variables will be defined in `src/app.css` to power the theme.

```css
/* In src/app.css */
@layer base {
  :root {
    /* Light Theme */
    --color-primary: 74 105 135;      /* #4A6987 */
    --color-primary-hover: 107 138 170; /* #6B8AAA */
    --color-primary-active: 58 83 108;  /* #3A536C */

    --color-bg-primary: 248 249 250;    /* #F8F9FA */
    --color-bg-secondary: 255 255 255;  /* #FFFFFF */
    --color-border: 233 236 239;      /* #E9ECEF */
    --color-text-primary: 33 37 41;      /* #212529 */
    --color-text-secondary: 108 117 125; /* #6C757D */
    
    --color-success: 47 158 68;        /* #2F9E44 */
    --color-warning: 247 180 44;       /* #F7B42C */
    --color-error: 214 57 57;         /* #D63939 */
  }

  .dark {
    /* Dark Theme */
    --color-primary: 107 151 194;     /* #6B97C2 */
    --color-primary-hover: 139 181 217;/* #8CB5D9 */
    --color-primary-active: 74 105 135; /* #4A6987 */

    --color-bg-primary: 26 29 33;       /* #1A1D21 */
    --color-bg-secondary: 44 49 56;      /* #2C3138 */
    --color-border: 55 60 68;         /* #373C44 */
    --color-text-primary: 248 249 250;   /* #F8F9FA */
    --color-text-secondary: 173 181 189; /* #ADB5BD */

    --color-success: 64 192 87;         /* #40C057 */
    --color-warning: 252 196 25;        /* #FCC419 */
    --color-error: 240 101 101;        /* #F06565 */
  }
}
```
