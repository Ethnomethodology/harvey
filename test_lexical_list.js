import { createEditor, ParagraphNode, TextNode } from 'lexical';
import { ListNode, ListItemNode } from '@lexical/list';

const editor = createEditor({
  nodes: [ListNode, ListItemNode, ParagraphNode, TextNode],
});

editor.update(() => {
  const li = new ListItemNode();
  console.log('li type:', typeof li);
  console.log('has getStyle:', typeof li.getStyle === 'function');
  console.log('has setStyle:', typeof li.setStyle === 'function');
});
