// src/lib/nodes/HorizontalRuleNode.js
import { DecoratorNode } from 'lexical';

export class HorizontalRuleNode extends DecoratorNode {
  static getType() {
    return 'horizontalrule';
  }

  static clone(node) {
    return new HorizontalRuleNode(node.__key);
  }

  static importJSON(serializedNode) {
    return new HorizontalRuleNode(serializedNode.key);
  }

  exportJSON() {
    return {
      type: 'horizontalrule',
      version: 1,
    };
  }

  createDOM(config) {
    const div = document.createElement('div');
    div.style.display = 'block';
    div.style.width = '100%';
    div.contentEditable = 'false';
    const hr = document.createElement('hr');
    hr.style.borderTop = '1px solid #ccc';
    hr.style.margin = '1em 0';
    div.appendChild(hr);
    return div;
  }

  updateDOM() {
    return false;
  }

  decorate() {
    return null;
  }

  isInline() {
      return false;
  }
}

export function $createHorizontalRuleNode() {
  return new HorizontalRuleNode();
}

export function $isHorizontalRuleNode(node) {
  return node instanceof HorizontalRuleNode;
}
