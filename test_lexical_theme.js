import { createEditor } from 'lexical';
const editor = createEditor({
  theme: {
    list: {
      nested: {
        listitem: 'my-nested-list-item'
      }
    }
  }
});
console.log(editor._config.theme.list.nested.listitem);
