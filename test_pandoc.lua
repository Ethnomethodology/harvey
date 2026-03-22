function Table(el)
  if el.attr then
    el.attr.classes:insert('Table')
  end
  return el
end
