import { DecoratorNode } from 'lexical';
import katex from 'katex';

export class EquationNode extends DecoratorNode {
  __equation;
  __inline;

  static getType() {
    return 'equation';
  }

  static clone(node) {
    return new EquationNode(node.__equation, node.__inline, node.__key);
  }

  static importDOM() {
    return {
      span: (domNode) => {
        if (domNode.classList.contains('math')) {
          return {
            conversion: convertEquationElement,
            priority: 1
          };
        }
        return null;
      },
      div: (domNode) => {
        if (domNode.classList.contains('math')) {
          return {
            conversion: convertEquationElement,
            priority: 1
          };
        }
        return null;
      }
    };
  }

  static importJSON(serializedNode) {
    const { equation, inline } = serializedNode;
    return new EquationNode(equation, inline);
  }

  exportJSON() {
    return {
      equation: this.__equation,
      inline: this.__inline,
      type: 'equation',
      version: 1
    };
  }

  constructor(equation, inline = true, key) {
    super(key);
    this.__equation = equation;
    this.__inline = inline;
  }

  createDOM(config) {
    const element = document.createElement(this.__inline ? 'span' : 'div');
    element.className = `lexical-equation-node ${this.__inline ? 'inline-block' : 'block text-center my-4'} cursor-pointer hover:bg-gray-100 dark:hover:bg-gray-800 rounded px-1 transition-colors`;
    element.contentEditable = 'false';
    element.dataset.lexicalEquation = this.__equation;

    try {
      katex.render(this.__equation, element, {
        displayMode: !this.__inline,
        throwOnError: false,
        errorColor: '#cc0000'
      });
    } catch (e) {
      element.innerText = `Error parsing equation: ${e.message}`;
      element.className += ' text-red-500 font-mono text-sm';
    }

    return element;
  }

  updateDOM(prevNode, dom, config) {
    if (prevNode.__equation !== this.__equation || prevNode.__inline !== this.__inline) {
      // Re-render KaTeX if the equation or inline mode changes
      try {
        dom.innerHTML = ''; // Clear existing content
        katex.render(this.__equation, dom, {
          displayMode: !this.__inline,
          throwOnError: false,
          errorColor: '#cc0000'
        });
        dom.className = `lexical-equation-node ${this.__inline ? 'inline-block' : 'block text-center my-4'} cursor-pointer hover:bg-gray-100 dark:hover:bg-gray-800 rounded px-1 transition-colors`;
        dom.dataset.lexicalEquation = this.__equation;
        // Adjust the DOM node tag if inline changes
        if ((prevNode.__inline ? 'span' : 'div') !== (this.__inline ? 'span' : 'div')) {
          return true; // Force recreate if tag changes
        }
      } catch (e) {
        dom.innerText = `Error parsing equation: ${e.message}`;
        dom.className = 'text-red-500 font-mono text-sm';
      }
    }
    return false;
  }

  decorate() {
    return null;
  }

  isInline() {
    return this.__inline;
  }
}

export function $createEquationNode(equation, inline) {
  return new EquationNode(equation, inline);
}

export function $isEquationNode(node) {
  return node instanceof EquationNode;
}

function convertEquationElement(domNode) {
  let equation = domNode.textContent;
  const inline = domNode.tagName === 'SPAN';

  // Strip Pandoc/KaTeX delimiters if present: \( ... \) or \[ ... \]
  equation = equation.replace(/^\s*\\\((.*)\\\)\s*$/s, '$1');
  equation = equation.replace(/^\s*\\\[(.*)\\\]\s*$/s, '$1');

  const node = $createEquationNode(equation, inline);
  return { node };
}
