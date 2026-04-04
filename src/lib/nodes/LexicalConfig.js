// src/lib/nodes/LexicalConfig.js
import { RootNode, ParagraphNode, TextNode, LineBreakNode } from 'lexical';
import { HeadingNode, QuoteNode } from '@lexical/rich-text';
import { CodeNode } from '@lexical/code';
import { ListNode, ListItemNode } from '@lexical/list';
import { TableNode, TableRowNode, TableCellNode } from '@lexical/table';
import { LinkNode } from '@lexical/link';

import {
  ExtendedTextNode,
  $createExtendedTextNode as _createExtendedTextNode
} from './ExtendedTextNode.js';
import { HorizontalRuleNode } from './HorizontalRuleNode.js';
import { ImageNode } from './ImageNode.js';
import { DateNode } from './DateNode.js';
import { EquationNode } from './EquationNode.js';

/**
 * Shared transformation function to upgrade a standard TextNode to an ExtendedTextNode
 * while faithfully preserving all formatting, style, and metadata.
 */
export const upgradeToExtendedTextNode = (node) => {
  const text = node.getTextContent();
  const extended = _createExtendedTextNode(text);

  // Faithfully copy all standard TextNode properties
  extended.__format = node.__format;
  extended.__style = node.__style;
  extended.__detail = node.__detail;
  extended.__mode = node.__mode;

  return extended;
};

/**
 * The standard set of nodes used across all Harvey Lexical editor instances.
 * This includes the faithful replacement of standard TextNode with ExtendedTextNode.
 */
export const SHARED_NODES = [
  ExtendedTextNode,
  {
    replace: TextNode,
    with: upgradeToExtendedTextNode
  },
  RootNode,
  ParagraphNode,
  LineBreakNode,
  HeadingNode,
  QuoteNode,
  CodeNode,
  ListNode,
  ListItemNode,
  LinkNode,
  TableNode,
  TableRowNode,
  TableCellNode,
  HorizontalRuleNode,
  ImageNode,
  DateNode,
  EquationNode
];
