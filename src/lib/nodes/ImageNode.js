import { DecoratorNode } from 'lexical';

export class ImageNode extends DecoratorNode {
  __filename;
  __altText;
  __width;
  __height;

  static getType() {
    return 'image';
  }

  static clone(node) {
    return new ImageNode(node.__filename, node.__altText, node.__width, node.__height, node.__key);
  }

  static importJSON(serializedNode) {
    const { filename, altText, width, height } = serializedNode;
    return $createImageNode(filename, altText, width, height);
  }

  exportJSON() {
    return {
      filename: this.getFilename(),
      altText: this.getAltText(),
      width: this.__width === 'inherit' ? undefined : this.__width,
      height: this.__height === 'inherit' ? undefined : this.__height,
      type: 'image',
      version: 1
    };
  }

  exportDOM(editor) {
    const { element } = super.exportDOM(editor) || {};
    const span = element || document.createElement('span');
    // Set to match createDOM, so html string has the img tag with the file data
    if (!element) {
      const img = document.createElement('img');
      img.setAttribute('data-filename', this.__filename);
      img.setAttribute('alt', this.__altText);
      span.appendChild(img);
    }
    return { element: span };
  }

  constructor(filename, altText, width, height, key) {
    super(key);
    this.__filename = filename;
    this.__altText = altText || 'Image';
    this.__width = width || 'inherit';
    this.__height = height || 'inherit';
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

    // Apply width and height
    if (this.__width !== 'inherit' && this.__width !== undefined) {
      img.style.width = typeof this.__width === 'number' ? `${this.__width}px` : this.__width;
    } else {
      img.style.maxWidth = '100%';
    }

    if (this.__height !== 'inherit' && this.__height !== undefined) {
      img.style.height = typeof this.__height === 'number' ? `${this.__height}px` : this.__height;
    } else {
      img.style.maxHeight = '500px';
    }

    img.style.objectFit = 'contain';
    img.style.cursor = 'default';

    span.appendChild(img);
    return span;
  }

  updateDOM(prevNode, dom, config) {
    const img = dom.firstChild;
    if (img) {
      if (prevNode.__filename !== this.__filename) {
        img.dataset.filename = this.__filename;
      }

      if (prevNode.__width !== this.__width) {
        if (this.__width !== 'inherit' && this.__width !== undefined) {
          img.style.width = typeof this.__width === 'number' ? `${this.__width}px` : this.__width;
          img.style.maxWidth = 'none';
        } else {
          img.style.width = '';
          img.style.maxWidth = '100%';
        }
      }

      if (prevNode.__height !== this.__height) {
        if (this.__height !== 'inherit' && this.__height !== undefined) {
          img.style.height =
            typeof this.__height === 'number' ? `${this.__height}px` : this.__height;
          img.style.maxHeight = 'none';
        } else {
          img.style.height = '';
          img.style.maxHeight = '500px';
        }
      }
    }
    return false;
  }

  getFilename() {
    return this.__filename;
  }

  getAltText() {
    return this.__altText;
  }

  setWidthAndHeight(width, height) {
    const writable = this.getWritable();
    writable.__width = width;
    writable.__height = height;
  }

  decorate() {
    // Return null since Svelte's Lexical integration just relies on DOM rendering here
    return null;
  }
}

export function $createImageNode(src, altText, width, height) {
  return new ImageNode(src, altText, width, height);
}

export function $isImageNode(node) {
  return node instanceof ImageNode;
}
