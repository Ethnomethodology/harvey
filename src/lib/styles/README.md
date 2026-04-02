# Styles (`src/lib/styles`)

**Purpose:** Contains global, application-wide CSS and SCSS stylesheets that are not scoped to individual Svelte components.

## Exported Utilities / Constants

- **`annotorious-global.css`**: Global overrides or base styles required for the Annotorious image annotation library.
- **`datepicker.css`**: Overrides and base styles for the Flowbite Datepicker component to match the application's aesthetic.
- **`tabulator-tailwind-theme.scss`**: A deeply customized SCSS file that adapts the core `tabulator-tables` grid library to utilize Tailwind CSS utility classes and match Harvey's specific dark/light mode UI requirements.

## Usage Example

These files are typically imported once at the root layout level (`src/routes/+layout.svelte`) to ensure they apply globally:

```svelte
<script>
  import '../app.css'; // Tailwind base
  import '$lib/styles/tabulator-tailwind-theme.scss';
</script>
```
