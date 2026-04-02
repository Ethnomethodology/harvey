# Lexical Custom Nodes (`src/lib/nodes`)

**Purpose:** Defines custom Lexical node classes that extend the core `@lexical/text` or `@lexical/decorator` capabilities, enabling specialized rich-text rendering like inline dates, equations, specialized text, and horizontal rules.

## Exported Nodes

- **`DateNode.js`**: A specialized inline node for rendering and formatting interactive dates within Lexical.
- **`EquationNode.js`**: A decorator node capable of rendering LaTeX or mathematical equations inline or as blocks.
- **`ExtendedTextNode.js`**: Extends the default text node to support application-specific custom data properties or formatting not handled by the core framework.
- **`HorizontalRuleNode.js`**: A decorator node to insert semantic thematic breaks (`<hr>`) within the document.
- **`ImageNode.js`**: A decorator node allowing for the insertion, resizing, and rendering of inline or block images inside the rich-text editor.

## Usage Example

```javascript
import { DateNode, $createDateNode } from '$lib/nodes/DateNode.js';

// Inside a Lexical update cycle:
editor.update(() => {
  const dateNode = $createDateNode('2024-05-20');
  $insertNodes([dateNode]);
});
```
