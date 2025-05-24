// src/lib/nodes/ExtendedTextNode.js
import {
  TextNode,
  $applyNodeReplacement as _applyNodeReplacement,
  $isTextNode as _isTextNode,
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
    clone.__highlightId = node.__highlightId;
    clone.__style = node.__style;
    return clone;
  }

  constructor(text, key) {
    super(text, key);
    this.__highlightId = null;
    this.__style = '';
  }

  // ---- highlight‑ID helpers -------------------------------------------------
  getHighlightId() {
    return this.getLatest().__highlightId;
  }
  setHighlightId(id) {
    this.getWritable().__highlightId = id ?? null;
    return this;
  }

  // ---- inline style helpers -------------------------------------------------
  setStyle(style = '') {
    this.getWritable().__style = style;
    return this;
  }
  getStyle() {
    // legacy serialisations may store the value on the instance but not initialise __style
    return this.__style || '';
  }

  // ---- DOM ------------------------------------------------------------------
  createDOM(config = { theme: {} }) { // default avoids “config.theme” error
    const dom = super.createDOM(config);
    if (this.__highlightId) dom.setAttribute('data-highlight-id', this.__highlightId);
    if (this.getStyle()) dom.setAttribute('style', this.getStyle());
    return dom;
  }

  updateDOM(prev, dom) {
    let changed = super.updateDOM(prev, dom);

    if (prev.__highlightId !== this.__highlightId) {
      if (this.__highlightId) dom.setAttribute('data-highlight-id', this.__highlightId);
      else dom.removeAttribute('data-highlight-id');
      changed = true;
    }

    if (prev.getStyle() !== this.getStyle()) {
      if (this.getStyle()) dom.setAttribute('style', this.getStyle());
      else dom.removeAttribute('style');
      changed = true;
    }

    return changed;
  }

  // ---- HTML export ----------------------------------------------------------
  exportDOM() {
    const span = document.createElement('span');
    if (this.__highlightId) span.setAttribute('data-highlight-id', this.__highlightId);
    if (this.getStyle()) span.setAttribute('style', this.getStyle());
    span.textContent = this.getTextContent();
    return { element: span };
  }

  // ---- JSON serialisation ---------------------------------------------------
  exportJSON() {
    return {
      ...super.exportJSON(),
      type: 'extended-text',
      highlightId: this.__highlightId,
      style: this.getStyle(),
      version: 1,
    };
  }

  static importJSON(serialised) {
    const node = new ExtendedTextNode(serialised.text);
    node.setFormat(serialised.format);
    if (serialised.detail !== undefined) node.setDetail(serialised.detail);
    if (serialised.mode !== undefined) node.setMode(serialised.mode);
    if (serialised.style !== undefined) node.setStyle(serialised.style);
    if (serialised.highlightId !== undefined) node.setHighlightId(serialised.highlightId);
    return node;
  }

  // ---- misc -----------------------------------------------------------------
  isSimpleText() {
    return (
      this.__type === 'extended-text' &&
      this.__mode === 0 &&
      this.__highlightId == null &&
      !this.getStyle()
    );
  }

  // keep existing importDOM override so we can parse style when pasting HTML
  static importDOM() {
    const importers = TextNode.importDOM();
    return {
      ...importers,
      span: () => ({
        conversion: patchStyleConversion(importers?.span),
        priority: 1,
      }),
      // retain overrides for <b>, <strong>, etc. if needed
      b: () => ({
        conversion: patchStyleConversion(importers?.b),
        priority: 1,
      }),
      strong: () => ({
        conversion: patchStyleConversion(importers?.strong),
        priority: 1,
      }),
      i: () => ({
        conversion: patchStyleConversion(importers?.i),
        priority: 1,
      }),
      em: () => ({
        conversion: patchStyleConversion(importers?.em),
        priority: 1,
      }),
      u: () => ({
        conversion: patchStyleConversion(importers?.u),
        priority: 1,
      }),
      s: () => ({
        conversion: patchStyleConversion(importers?.s),
        priority: 1,
      }),
      sub: () => ({
        conversion: patchStyleConversion(importers?.sub),
        priority: 1,
      }),
      sup: () => ({
        conversion: patchStyleConversion(importers?.sup),
        priority: 1,
      }),
      code: () => ({
        conversion: patchStyleConversion(importers?.code),
        priority: 1,
      }),
    };
  }
}

// ----- helpers ---------------------------------------------------------------
export function $createExtendedTextNode(text = '') {
  return _applyNodeReplacement(new ExtendedTextNode(text));
}
export function $isExtendedTextNode(node) {
  return node instanceof ExtendedTextNode;
}

/**
 * Patch a DOM‑to‑Lexical converter so it copies inline style attributes onto
 * the resulting TextNode (background‑color, colour, etc.).
 */
function patchStyleConversion(originalDOMConverter) {
  return (htmlElementNode) => {
    const original = originalDOMConverter?.(htmlElementNode);

    // Run any existing conversion to get a base Lexical node
    const baseNode =
      original?.conversion?.(htmlElementNode)?.node ??
      original?.node ??
      $createExtendedTextNode(htmlElementNode.textContent ?? '');

    // Ensure we end up with an ExtendedTextNode instance
    let node = baseNode;
    if (_isTextNode(node) && !(node instanceof ExtendedTextNode)) {
      const upgraded = $createExtendedTextNode(node.getTextContent());
      upgraded.setFormat(node.getFormat());
      node = upgraded;
    }

    // Pull selected inline styles from the HTML element
    const styles = [];
    const s = htmlElementNode.style;
    if (s.backgroundColor) styles.push(`background-color: ${s.backgroundColor}`);
    if (s.color) styles.push(`color: ${s.color}`);
    if (s.fontFamily) styles.push(`font-family: ${s.fontFamily}`);
    if (s.fontWeight) styles.push(`font-weight: ${s.fontWeight}`);
    if (s.fontSize) styles.push(`font-size: ${s.fontSize}`);
    if (s.textDecoration) styles.push(`text-decoration: ${s.textDecoration}`);

    const styleString = styles.filter(Boolean).join('; ');
    if (styleString) node.setStyle(styleString);

    // copy highlight id if present
    const hId = htmlElementNode.getAttribute('data-highlight-id');
    if (hId && typeof node.setHighlightId === 'function') node.setHighlightId(hId);

    return { node };
  };
}