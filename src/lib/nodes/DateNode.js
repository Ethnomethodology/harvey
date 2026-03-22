// src/lib/nodes/DateNode.js
import { DecoratorNode } from 'lexical';

/**
 * Node for interactive dates that opens a picker when clicked.
 */
export class DateNode extends DecoratorNode {
  __date; // ISO or formatted string
  __format;
  __showTime;
  __timeFormat;
  __displayValue; // The actual text shown in the editor

  static getType() {
    return 'date';
  }

  static clone(node) {
    return new DateNode(
      node.__date,
      node.__format,
      node.__showTime,
      node.__timeFormat,
      node.__displayValue,
      node.__key
    );
  }

  static importJSON(serializedNode) {
    const { date, format, showTime, timeFormat, displayValue } = serializedNode;
    return new DateNode(date, format, showTime, timeFormat, displayValue);
  }

  exportJSON() {
    return {
      date: this.__date,
      format: this.__format,
      showTime: this.__showTime,
      timeFormat: this.__timeFormat,
      displayValue: this.__displayValue,
      type: 'date',
      version: 1,
    };
  }

  constructor(date, format, showTime, timeFormat, displayValue, key) {
    super(key);
    this.__date = date;
    this.__format = format;
    this.__showTime = showTime;
    this.__timeFormat = timeFormat;
    this.__displayValue = displayValue || date;
  }

  createDOM(config) {
    const span = document.createElement('span');
    span.contentEditable = 'false';
    span.className = 'lexical-date-node inline-flex items-center gap-1.5 px-2.5 py-1 rounded-md bg-blue-50 text-blue-700 dark:bg-blue-900/30 dark:text-blue-300 border border-blue-200 dark:border-blue-800 font-medium cursor-pointer hover:bg-blue-100 dark:hover:bg-blue-900/50 transition-colors my-0.5 align-baseline text-sm';
    span.innerText = this.__displayValue;
    
    // Add a small calendar icon implicitly via pseudo-elements or just keep it clean
    // For now keep it as a clean pill
    
    return span;
  }

  updateDOM(prevNode, dom, config) {
    if (prevNode.__displayValue !== this.__displayValue) {
      dom.innerText = this.__displayValue;
    }
    return false;
  }

  decorate() {
    return null;
  }

  isInline() {
    return true;
  }
}

export function $createDateNode(date, format, showTime, timeFormat, displayValue) {
  return new DateNode(date, format, showTime, timeFormat, displayValue);
}

export function $isDateNode(node) {
  return node instanceof DateNode;
}
