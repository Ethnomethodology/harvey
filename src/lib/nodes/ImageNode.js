import { DecoratorNode } from 'lexical';

export class ImageNode extends DecoratorNode {
  __filename;
  __altText;

  static getType() {
    return 'image';
  }

  static clone(node) {
    return new ImageNode(node.__filename, node.__altText, node.__key);
  }

  static importJSON(serializedNode) {
    const { filename, altText } = serializedNode;
    return $createImageNode(filename, altText);
  }

  exportJSON() {
    return {
      filename: this.getFilename(),
      altText: this.getAltText(),
      type: 'image',
      version: 1,
    };
  }

  constructor(filename, altText, key) {
    super(key);
    this.__filename = filename;
    this.__altText = altText || 'Image';
  }

  createDOM(config) {
    const span = document.createElement('span');
    span.style.display = 'block';
    span.style.textAlign = 'center';
    span.style.margin = '10px 0';
    span.style.maxWidth = '100%';
    span.style.position = 'relative';
    span.className = 'editor-image-wrapper';

    const img = document.createElement('img');

    // We store the filename so the frontend can asynchronously resolve and inject the `asset://` src.
    img.dataset.filename = this.__filename;
    img.alt = this.__altText;
    img.style.maxWidth = '100%';
    img.style.maxHeight = '500px';
    img.style.objectFit = 'contain';
    img.style.cursor = 'default';

    span.appendChild(img);
    return span;
  }

  updateDOM(prevNode, dom, config) {
    const img = dom.firstChild;
    if (img && prevNode.__filename !== this.__filename) {
        img.dataset.filename = this.__filename;
    }
    return false;
  }

  getFilename() {
    return this.__filename;
  }

  getAltText() {
    return this.__altText;
  }

  decorate() {
    // Return null since Svelte's Lexical integration just relies on DOM rendering here
    return null;
  }
}

export function $createImageNode(src, altText) {
  return new ImageNode(src, altText);
}

export function $isImageNode(node) {
  return node instanceof ImageNode;
}
