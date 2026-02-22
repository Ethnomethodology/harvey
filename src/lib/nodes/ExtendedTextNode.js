// src/lib/nodes/ExtendedTextNode.js
import {
  TextNode,
  $applyNodeReplacement as _applyNodeReplacement,
  $isTextNode as _isTextNode,
  // LexicalConstants
  IS_BOLD,
  IS_ITALIC,
  IS_STRIKETHROUGH,
  IS_UNDERLINE,
  IS_CODE,
  IS_SUBSCRIPT,
  IS_SUPERSCRIPT,
  IS_HIGHLIGHT, // Assuming this is also a constant you might use or is standard
} from 'lexical';

/**
 * ExtendedTextNode
 * ----------------
 * A Text node that can store:
 *   • an arbitrary inline CSS style string (for highlight colours, etc.)
 *   • an optional highlight‑ID (data‑attribute for UI interactions)
 *
 * It knows how to:
 *   • render itself to the DOM (createDOM / updateDOM)
 *   • export itself to HTML (exportDOM) as <span style="…">text</span>
 *   • serialise to / from Lexical JSON (exportJSON / importJSON)
 */
export class ExtendedTextNode extends TextNode {
  /** @type {string|null} */
  __highlightId;
  /** @type {string} */
  __style;

  // ---- basic ----------------------------------------------------------------
  static getType() {
    return 'extended-text';
  }

  static clone(node) {
    const clone = new ExtendedTextNode(node.__text, node.__key);
    clone.__format = node.__format; // Ensure format is cloned
    clone.__highlightId = node.__highlightId;
    clone.__style = node.__style;
    // Clone detail and mode if they exist on the source node, TextNode handles these
    clone.__detail = node.__detail;
    clone.__mode = node.__mode;
    return clone;
  }

  constructor(text, key) {
    super(text, key);
    this.__highlightId = null;
    this.__style = '';
  }

  // ---- highlight‑ID helpers -------------------------------------------------
  getHighlightId() {
    const latest = this.getLatest();
    return latest.__highlightId;
  }
  setHighlightId(id) {
    const writable = this.getWritable();
    writable.__highlightId = id ?? null;
    return writable; // Lexical convention is to return the writable node
  }

  // ---- inline style helpers -------------------------------------------------
  setStyle(style = '') {
    const writable = this.getWritable();
    writable.__style = style;
    return writable; // Lexical convention
  }
  getStyle() {
    const latest = this.getLatest();
    return latest.__style || '';
  }

  // ---- DOM ------------------------------------------------------------------
  createDOM(config, editor) { // Added editor instance for consistency, though not strictly used by super.createDOM for TextNode
    const dom = super.createDOM(config, editor); // Pass editor if super expects it
    const latest = this.getLatest();
    if (latest.__highlightId) dom.setAttribute('data-highlight-id', latest.__highlightId);

    const isBold = latest.hasFormat('bold');
    const style = getEffectiveStyle(latest.getStyle(), isBold);

    if (style) dom.setAttribute('style', style);
    return dom;
  }

  updateDOM(prevNode, dom, config) { // Correctly accept EditorConfig
    let changed = super.updateDOM(prevNode, dom, config); // Pass config to super
    const latest = this.getLatest();

    if (prevNode.__highlightId !== latest.__highlightId) {
      if (latest.__highlightId) dom.setAttribute('data-highlight-id', latest.__highlightId);
      else dom.removeAttribute('data-highlight-id');
      changed = true;
    }

    const prevIsBold = prevNode.hasFormat('bold');
    const nextIsBold = latest.hasFormat('bold');

    const prevStyle = getEffectiveStyle(prevNode.getStyle(), prevIsBold);
    const nextStyle = getEffectiveStyle(latest.getStyle(), nextIsBold);

    if (prevStyle !== nextStyle) {
      if (nextStyle) dom.setAttribute('style', nextStyle);
      else dom.removeAttribute('style');
      changed = true;
    }
    return changed;
  }

  // ---- HTML export ----------------------------------------------------------
  exportDOM(editor) { // editor instance is passed by $generateHtmlFromNodes
    let element = document.createElement('span');
    const latest = this.getLatest();

    // Apply custom styles and highlight ID
    if (latest.__highlightId) {
      element.setAttribute('data-highlight-id', latest.__highlightId);
    }

    const isBold = latest.hasFormat('bold');
    const styleAttribute = getEffectiveStyle(latest.getStyle(), isBold);

    if (styleAttribute) {
      element.setAttribute('style', styleAttribute);
    }

    element.textContent = latest.getTextContent(); // Set text content

    // Apply standard Lexical formats by wrapping the element if necessary
    // This creates nested elements like <b><i>text</i></b>
    // Order of application might matter for some CSS rules, but typical browser handling is fine.
    const format = latest.__format;

    if (format & IS_BOLD) {
      const strong = document.createElement('strong');
      strong.appendChild(element);
      element = strong;
    }
    if (format & IS_ITALIC) {
      const em = document.createElement('em');
      em.appendChild(element);
      element = em;
    }
    if (format & IS_STRIKETHROUGH) {
      const s = document.createElement('s');
      s.appendChild(element);
      element = s;
    }
    if (format & IS_UNDERLINE) {
      const u = document.createElement('u');
      u.appendChild(element);
      element = u;
    }
    if (format & IS_CODE) {
      const code = document.createElement('code');
      // Lexical's CodeNode often has specific theme classes; ExtendedTextNode with IS_CODE format
      // might not pick those up directly here unless style is also set.
      // For HTML export, <code>text</code> is standard.
      // If specific styling for inline code is needed via theme,
      // it's usually handled by the theme applied to the <span> by Lexical's core TextNode.
      // Our custom style attribute on the span would take precedence or add to it.
      code.appendChild(element);
      element = code;
    }
    if (format & IS_SUBSCRIPT) {
      const sub = document.createElement('sub');
      sub.appendChild(element);
      element = sub;
    }
    if (format & IS_SUPERSCRIPT) {
      const sup = document.createElement('sup');
      sup.appendChild(element);
      element = sup;
    }
    // IS_HIGHLIGHT is a custom bitmask here, primarily for `ExtendedTextNode`
    // The visual highlighting is done by setting 'background-color' in the style string.
    // No standard HTML tag for generic highlight, so <mark> or styled <span> is common.
    // Since we use `style` attribute, `mark` might be redundant if bg-color is already set.
    // If IS_HIGHLIGHT implies a specific class OR you want <mark> regardless of style string:
    // if (format & IS_HIGHLIGHT) {
    //   const mark = document.createElement('mark');
    //   mark.appendChild(element);
    //   element = mark;
    // }


    return { element };
  }

  // ---- JSON serialisation ---------------------------------------------------
  exportJSON() {
    return {
      ...super.exportJSON(), // Includes text, format, detail, mode, version (from TextNode)
      type: 'extended-text', // Override type
      highlightId: this.getLatest().__highlightId,
      style: this.getLatest().getStyle(),
      // version for ExtendedTextNode specific properties, TextNode's version handles its own.
      // If ExtendedTextNode itself evolves, increment this.
      // For now, keeping it simple or aligning with TextNode's version if not adding new persistent fields.
      // version: 1, // Example if ExtendedTextNode itself had versions for its own fields
    };
  }

  static importJSON(serializedNode) {
    // Preserve the original key to allow Lexical to reconcile nodes correctly
    const node = new ExtendedTextNode(serializedNode.text, serializedNode.key);
    node.setFormat(serializedNode.format);
    node.setDetail(serializedNode.detail);
    node.setMode(serializedNode.mode);
    node.setStyle(serializedNode.style || '');
    node.setHighlightId(serializedNode.highlightId || null);
    return node;
  }


  // ---- misc -----------------------------------------------------------------
  isSimpleText() {
    const latest = this.getLatest();
    return (
      latest.__type === 'extended-text' &&
      latest.__mode === 0 && // Assuming 0 is the "normal" mode for TextNode
      latest.__highlightId == null &&
      !latest.getStyle() &&
      latest.__format === 0 // No standard formats applied
    );
  }

  // keep existing importDOM override so we can parse style when pasting HTML
  static importDOM() {
    const importers = TextNode.importDOM();
    return {
      ...importers, // Keep original importers for things like text content from various tags
      span: (node) => {
        // If the original span importer exists, try to use it and then patch
        if (importers && typeof importers.span === 'function') {
          const original = importers.span(node);
          if (original && original.conversion) {
            return {
              conversion: patchStyleConversion(original.conversion),
              priority: 1, // Ensure this runs
            };
          }
        }
        // Fallback if original span importer is not found or not structured as expected
        return {
          conversion: patchStyleConversion(() => ({ node: $createExtendedTextNode(node.textContent) })),
          priority: 1,
        };
      },
      // Standard format tags should ideally be handled by Lexical's core TextNode.importDOM(),
      // which correctly sets the format flags. We then upgrade to ExtendedTextNode.
      // However, if we want to directly intercept and ensure they become ExtendedTextNodes
      // with their styles also parsed, we can override them here.
      // For now, relying on TextNode's format parsing and then style patching.
      // If TextNode.importDOM() returns null for these, we'd need explicit handlers.
      b: (node) => ({
          conversion: patchStyleConversion(importers?.b || (() => ({node: $createExtendedTextNode(node.textContent).setFormat(IS_BOLD)}))),
          priority: 1,
      }),
      strong: (node) => ({
          conversion: patchStyleConversion(importers?.strong || (() => ({node: $createExtendedTextNode(node.textContent).setFormat(IS_BOLD)}))),
          priority: 1,
      }),
      i: (node) => ({
          conversion: patchStyleConversion(importers?.i || (() => ({node: $createExtendedTextNode(node.textContent).setFormat(IS_ITALIC)}))),
          priority: 1,
      }),
      em: (node) => ({
          conversion: patchStyleConversion(importers?.em || (() => ({node: $createExtendedTextNode(node.textContent).setFormat(IS_ITALIC)}))),
          priority: 1,
      }),
      u: (node) => ({
          conversion: patchStyleConversion(importers?.u || (() => ({node: $createExtendedTextNode(node.textContent).setFormat(IS_UNDERLINE)}))),
          priority: 1,
      }),
      s: (node) => ({
          conversion: patchStyleConversion(importers?.s || (() => ({node: $createExtendedTextNode(node.textContent).setFormat(IS_STRIKETHROUGH)}))),
          priority: 1,
      }),
      sub: (node) => ({
          conversion: patchStyleConversion(importers?.sub || (() => ({node: $createExtendedTextNode(node.textContent).setFormat(IS_SUBSCRIPT)}))),
          priority: 1,
      }),
      sup: (node) => ({
          conversion: patchStyleConversion(importers?.sup || (() => ({node: $createExtendedTextNode(node.textContent).setFormat(IS_SUPERSCRIPT)}))),
          priority: 1,
      }),
      code: (node) => ({ // Inline code
          conversion: patchStyleConversion(importers?.code || (() => ({node: $createExtendedTextNode(node.textContent).setFormat(IS_CODE)}))),
          priority: 1,
      }),
      font: (node) => { // Handle <font color="..."> for pasted content
        const color = node.getAttribute('color');
        const baseConversion = importers?.span || (() => ({ node: $createExtendedTextNode(node.textContent) }));
        const conversion = patchStyleConversion(baseConversion);
        return (htmlElementNode) => {
          const result = conversion(htmlElementNode);
          if (result.node && color) {
            const existingStyle = result.node.getStyle() || '';
            const newStyle = `${existingStyle}${existingStyle ? ';' : ''}color: ${color}`;
            result.node.setStyle(newStyle);
          }
          return result;
        };
      },
      mark: (node) => { // Handle <mark> for pasted content (often used for highlights)
        const baseConversion = importers?.span || (() => ({ node: $createExtendedTextNode(node.textContent) }));
        const conversion = patchStyleConversion(baseConversion);
        return (htmlElementNode) => {
            const result = conversion(htmlElementNode);
            if (result.node) {
                // If you have a default highlight color or want to parse a specific class:
                const existingStyle = result.node.getStyle() || '';
                const newStyle = `${existingStyle}${existingStyle ? ';' : ''}background-color: yellow;`; // Default to yellow for <mark>
                result.node.setStyle(newStyle);
                result.node.setFormat(result.node.getFormat() | IS_HIGHLIGHT); // Optionally set IS_HIGHLIGHT flag
            }
            return result;
        };
      },
    };
  }
}

// ----- helpers ---------------------------------------------------------------
export function $createExtendedTextNode(text = '', key) {
  return new ExtendedTextNode(text, key);
}
export function $isExtendedTextNode(node) {
  return node instanceof ExtendedTextNode;
}

function getEffectiveStyle(style, isBold) {
  if (!style) return '';
  if (isBold) {
    // Remove font-weight from inline style if bold formatting is active,
    // to allow the bold class/tag to take precedence.
    // Handles 'font-weight: ...;' with potential spaces.
    return style.replace(/(^|;)\s*font-weight\s*:[^;]+(;|$)/gi, '$1').replace(/^;+/, '').replace(/;+$/, '');
  }
  return style;
}

/**
 * Patch a DOM‑to‑Lexical converter so it copies inline style attributes onto
 * the resulting TextNode (background‑color, colour, etc.).
 * Ensures the node is an ExtendedTextNode.
 */
function patchStyleConversion(originalDOMConverter) {
  return (htmlElementNode) => {
    // Get the base conversion from the original converter
    let conversionResult;
    if (typeof originalDOMConverter === 'function') {
        // It might be a function that directly returns the node object { node: ... }
        // or a function that returns a converter object { conversion: (el) => ({ node: ... }) }
        const tempResult = originalDOMConverter(htmlElementNode);
        if (tempResult && typeof tempResult.conversion === 'function') {
            conversionResult = tempResult.conversion(htmlElementNode);
        } else if (tempResult && tempResult.node) {
            conversionResult = tempResult;
        } else {
            // Fallback if the original converter is unhelpful or doesn't produce a node
            conversionResult = { node: $createExtendedTextNode(htmlElementNode.textContent ?? '') };
        }
    } else {
        // Fallback if originalDOMConverter is not a function (e.g., null or undefined)
        conversionResult = { node: $createExtendedTextNode(htmlElementNode.textContent ?? '') };
    }

    let lexicalNode = conversionResult.node;

    // Ensure we're working with an ExtendedTextNode
    if (lexicalNode && _isTextNode(lexicalNode) && !(lexicalNode instanceof ExtendedTextNode)) {
      const textContent = lexicalNode.getTextContent();
      const format = lexicalNode.getFormat();
      const detail = lexicalNode.getDetail();
      const mode = lexicalNode.getMode();

      const upgradedNode = $createExtendedTextNode(textContent);
      upgradedNode.setFormat(format);
      upgradedNode.setDetail(detail);
      upgradedNode.setMode(mode);
      lexicalNode = upgradedNode;
    } else if (!lexicalNode) {
      // If no node was created by the original, create a fresh ExtendedTextNode
      lexicalNode = $createExtendedTextNode(htmlElementNode.textContent ?? '');
    }

    // At this point, lexicalNode should be an ExtendedTextNode instance
    if (lexicalNode instanceof ExtendedTextNode) {
      // Pull selected inline styles from the HTML element
      const styles = [];
      const s = htmlElementNode.style;
      if (s) { // Ensure htmlElementNode.style exists
          if (s.backgroundColor) styles.push(`background-color: ${s.backgroundColor}`);
          if (s.color) styles.push(`color: ${s.color}`);
          if (s.fontFamily) styles.push(`font-family: ${s.fontFamily}`);
          if (s.fontWeight) styles.push(`font-weight: ${s.fontWeight}`);
          if (s.fontSize) styles.push(`font-size: ${s.fontSize}`);
          // text-decoration is usually handled by format flags, but can be explicit
          if (s.textDecoration) styles.push(`text-decoration: ${s.textDecoration}`);
      }


      const styleString = styles.filter(Boolean).join('; ');
      if (styleString) {
        const existingStyle = lexicalNode.getStyle() || '';
        lexicalNode.setStyle(existingStyle ? `${existingStyle}; ${styleString}` : styleString);
      }

      // Copy highlight id if present
      const hId = htmlElementNode.getAttribute('data-highlight-id');
      if (hId) {
        lexicalNode.setHighlightId(hId);
      }
    } else {
        console.warn("patchStyleConversion: lexicalNode is not an ExtendedTextNode after potential upgrade.", lexicalNode);
    }

    return { node: lexicalNode };
  };
}