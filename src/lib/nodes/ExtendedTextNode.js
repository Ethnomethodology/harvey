// src/lib/nodes/ExtendedTextNode.js
import {
	$applyNodeReplacement as _applyNodeReplacement, // Aliased
	$isTextNode as _isTextNode, // Aliased
	TextNode,
	$getNodeByKey as _getNodeByKey, // Aliased
} from 'lexical';



export class ExtendedTextNode extends TextNode {
	__highlightId;

	static getType() {
		return 'extended-text';
	}

	static clone(node) {
		const newInstance = new ExtendedTextNode(node.__text, node.__key);
		newInstance.__highlightId = node.__highlightId;
		return newInstance;
	}

	constructor(text, key) {
		super(text, key);
		this.__highlightId = null;
	}

	getHighlightId() {
		const self = this.getLatest();
		return self.__highlightId;
	}

	setHighlightId(id) {
		const self = this.getWritable();
		self.__highlightId = id;
	}

	createDOM(config, editor) {
		const dom = super.createDOM(config, editor);
		if (this.__highlightId) {
			dom.setAttribute('data-highlight-id', this.__highlightId);
		}
		return dom;
	}

	updateDOM(prevNode, dom, config) {
		const isUpdated = super.updateDOM(prevNode, dom, config);
		if (prevNode.__highlightId !== this.__highlightId) {
			if (this.__highlightId) {
				dom.setAttribute('data-highlight-id', this.__highlightId);
			} else {
				dom.removeAttribute('data-highlight-id');
			}
		}
		return isUpdated;
	}

	static importJSON(serializedNode) {
		const node = $createExtendedTextNode(serializedNode.text); // Using our factory function
		node.setFormat(serializedNode.format);
		node.setDetail(serializedNode.detail);
		node.setMode(serializedNode.mode);
		node.setStyle(serializedNode.style);
		if (serializedNode.highlightId !== undefined) {
			 node.setHighlightId(serializedNode.highlightId);
		}
		return node;
	}

	exportJSON() {
		const textNodeJSON = super.exportJSON();
		return {
			...textNodeJSON,
			type: 'extended-text',
			highlightId: this.__highlightId,
			version: 1, // Explicitly ensure version if not handled by super
		};
	}

	static importDOM() {
	  const importers = TextNode.importDOM();
	  return {
		...importers,
		code: () => ({
		  conversion: patchStyleConversion(importers?.code),
		  priority: 1
		}),
		em: () => ({
		  conversion: patchStyleConversion(importers?.em),
		  priority: 1
		}),
		i: () => ({
		  conversion: patchStyleConversion(importers?.i),
		  priority: 1
		}),
		b: () => ({
		  conversion: patchStyleConversion(importers?.b),
		  priority: 1
		}),
		span: () => ({ // Keep span override for potential data-attribute parsing if needed later
		  conversion: patchStyleConversion(importers?.span),
		  priority: 1
		}),
		strong: () => ({
		  conversion: patchStyleConversion(importers?.strong),
		  priority: 1
		}),
		sub: () => ({
		  conversion: patchStyleConversion(importers?.sub),
		  priority: 1
		}),
		sup: () => ({
		  conversion: patchStyleConversion(importers?.sup),
		  priority: 1
		}),
		 u: () => ({
		  conversion: patchStyleConversion(importers?.u),
		  priority: 1
		}),
		 s: () => ({
		  conversion: patchStyleConversion(importers?.s),
		  priority: 1
		}),
	  };
	}

	isSimpleText() {
	  return (this.__type === 'extended-text' && this.__mode === 0 && this.__highlightId === null);
	}
}

export function $createExtendedTextNode(text = '') {
	return _applyNodeReplacement(new ExtendedTextNode(text)); // Use aliased import
}

export function $isExtendedTextNode(node) { // Export name is fine, usage in Svelte will alias if needed
	  return node instanceof ExtendedTextNode;
}

function patchStyleConversion(originalDOMConverter) {
	return (htmlElementNode) => {
	  const original = originalDOMConverter?.(htmlElementNode);
	  const originalConversionFn = original && typeof original.conversion === 'function' ? original.conversion : null;
	  const originalConversionOutput = originalConversionFn ? originalConversionFn(htmlElementNode) : { node: null };
	  const baseOutput = originalConversionOutput || { node: null };

	  const stylePatchingForChild = (lexicalNode, parentLexicalNode) => {
		const originalForChildFn = baseOutput && typeof baseOutput.forChild === 'function' ? baseOutput.forChild : ((ln) => ln);
		const resultFromOriginalForChild = originalForChildFn(lexicalNode, parentLexicalNode);

		if (_isTextNode(resultFromOriginalForChild)) { // Use aliased import
		  const styles = [];
		  if (htmlElementNode.style) {
			  if (htmlElementNode.style.backgroundColor) styles.push(`background-color: ${htmlElementNode.style.backgroundColor}`);
			  if (htmlElementNode.style.color) styles.push(`color: ${htmlElementNode.style.color}`);
			  if (htmlElementNode.style.fontFamily) styles.push(`font-family: ${htmlElementNode.style.fontFamily}`);
			  if (htmlElementNode.style.fontWeight) styles.push(`font-weight: ${htmlElementNode.style.fontWeight}`);
			  if (htmlElementNode.style.fontSize) styles.push(`font-size: ${htmlElementNode.style.fontSize}`);
			  if (htmlElementNode.style.textDecoration) styles.push(`text-decoration: ${htmlElementNode.style.textDecoration}`);
		  }
		  const styleString = styles.filter(Boolean).join('; ');
		  if (styleString.length > 0) {
			if (typeof resultFromOriginalForChild.setStyle === 'function') {
			   return resultFromOriginalForChild.setStyle(styleString);
			}
		  }
		}
		return resultFromOriginalForChild;
	  };

	  return {
		...baseOutput,
		conversion: (nodeWithFormat) => {
			let outputNode = null;
			if (baseOutput && typeof baseOutput.conversion === 'function') {
				const conversionResult = baseOutput.conversion(nodeWithFormat);
				outputNode = conversionResult?.node;
			} else if (baseOutput && baseOutput.node) {
				outputNode = baseOutput.node;
			} else {
				outputNode = $createExtendedTextNode(nodeWithFormat.textContent || '');
			}

			if (outputNode && TextNode.isTextNode(outputNode) && !(outputNode instanceof ExtendedTextNode)) {
				const upgradedNode = $createExtendedTextNode(outputNode.getTextContent());
				upgradedNode.setFormat(outputNode.getFormat());
				upgradedNode.setStyle(outputNode.getStyle());
				outputNode = upgradedNode;
			}

			if (outputNode instanceof ExtendedTextNode) {
				const highlightId = htmlElementNode.getAttribute('data-highlight-id');
				if (highlightId) {
					outputNode.setHighlightId(highlightId);
				}
			}
			return { node: outputNode };
		},
		forChild: stylePatchingForChild,
		priority: original?.priority > 0 ? original.priority : 1,
	  };
	};
}