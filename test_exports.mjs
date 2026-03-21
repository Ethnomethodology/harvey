import * as list from './node_modules/@lexical/list/LexicalList.node.mjs';
console.log(Object.keys(list).filter(k => k.toLowerCase().includes('check')));
