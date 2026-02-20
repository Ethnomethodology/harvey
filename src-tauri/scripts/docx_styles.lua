function Span(el)
  -- Parse styles from data- attributes which we will populate in Rust

  local color = el.attributes['data-color']
  local bg_color = el.attributes['data-bg-color']
  local font_family = el.attributes['data-font-family']
  local font_size = el.attributes['data-font-size']

  if not (color or bg_color or font_family or font_size) then
    return nil
  end

  return process_content(el.content, {
    color = color,
    bg_color = bg_color,
    font_family = font_family,
    font_size = font_size
  })
end

function process_content(content, style_overrides)
  local result = {}
  for _, item in ipairs(content) do
    if item.t == 'Str' or item.t == 'Space' then
      table.insert(result, create_openxml_run(item, style_overrides))
    elseif item.t == 'Strong' then
      local sub_style = shallow_copy(style_overrides)
      sub_style.bold = true
      local sub_res = process_content(item.content, sub_style)
      for _, r in ipairs(sub_res) do table.insert(result, r) end
    elseif item.t == 'Emph' then
      local sub_style = shallow_copy(style_overrides)
      sub_style.italic = true
      local sub_res = process_content(item.content, sub_style)
      for _, r in ipairs(sub_res) do table.insert(result, r) end
    elseif item.t == 'Underline' then
      local sub_style = shallow_copy(style_overrides)
      sub_style.underline = true
      local sub_res = process_content(item.content, sub_style)
      for _, r in ipairs(sub_res) do table.insert(result, r) end
    elseif item.t == 'Strikeout' then
      local sub_style = shallow_copy(style_overrides)
      sub_style.strike = true
      local sub_res = process_content(item.content, sub_style)
      for _, r in ipairs(sub_res) do table.insert(result, r) end
    elseif item.t == 'Superscript' then
        local sub_style = shallow_copy(style_overrides)
        sub_style.vertAlign = "superscript"
        local sub_res = process_content(item.content, sub_style)
        for _, r in ipairs(sub_res) do table.insert(result, r) end
    elseif item.t == 'Subscript' then
        local sub_style = shallow_copy(style_overrides)
        sub_style.vertAlign = "subscript"
        local sub_res = process_content(item.content, sub_style)
        for _, r in ipairs(sub_res) do table.insert(result, r) end
    elseif item.t == 'Span' then
       -- Nested spans? Recurse with merged styles if needed, or just pass through
       local sub_res = process_content(item.content, style_overrides)
       for _, r in ipairs(sub_res) do table.insert(result, r) end
    else
      -- Fallback for others
      table.insert(result, item)
    end
  end
  return result
end

function create_openxml_run(item, styles)
  local text = ""
  if item.t == 'Str' then text = item.text
  elseif item.t == 'Space' then text = " " end

  -- Strict OpenXML Order for w:rPr children:
  -- 1. rFonts
  -- 2. b
  -- 3. i
  -- 4. strike
  -- 5. color
  -- 6. sz, szCs
  -- 7. highlight
  -- 8. u
  -- 9. shd
  -- 10. vertAlign

  local xml = '<w:r>'
  xml = xml .. '<w:rPr>'

  -- 1. rFonts
  if styles.font_family then
    xml = xml .. '<w:rFonts w:ascii="' .. styles.font_family .. '" w:hAnsi="' .. styles.font_family .. '" w:cs="' .. styles.font_family .. '" w:eastAsia="' .. styles.font_family .. '"/>'
  end

  -- 2. b
  if styles.bold then xml = xml .. '<w:b/>' end

  -- 3. i
  if styles.italic then xml = xml .. '<w:i/>' end

  -- 4. strike
  if styles.strike then xml = xml .. '<w:strike/>' end

  -- 5. color
  if styles.color then
    local c = styles.color:gsub('#', '')
    xml = xml .. '<w:color w:val="' .. c .. '"/>'
  end

  -- 6. sz
  if styles.font_size then
    local size_val_str = styles.font_size:gsub('pt', '')
    local is_px = size_val_str:find('px')
    size_val_str = size_val_str:gsub('px', '')

    local size_num = tonumber(size_val_str)
    if size_num then
        local half_points
        if is_px then
            -- 1px ~= 0.75pt. 1pt = 2 half-points.
            -- 1px = 1.5 half-points.
            half_points = math.floor(size_num * 1.5)
        else
            half_points = math.floor(size_num * 2)
        end
        xml = xml .. '<w:sz w:val="' .. half_points .. '"/>'
        xml = xml .. '<w:szCs w:val="' .. half_points .. '"/>'
    end
  end

  -- 7. highlight
  if styles.bg_color then
    local c = styles.bg_color:gsub('#', '')
    if c:lower() == "ffff00" or c:lower() == "yellow" then
        xml = xml .. '<w:highlight w:val="yellow"/>'
    elseif c:lower() == "00ff00" or c:lower() == "lime" then
        xml = xml .. '<w:highlight w:val="green"/>'
    elseif c:lower() == "00ffff" or c:lower() == "cyan" then
        xml = xml .. '<w:highlight w:val="cyan"/>'
    elseif c:lower() == "ff00ff" or c:lower() == "magenta" then
        xml = xml .. '<w:highlight w:val="magenta"/>'
    end
  end

  -- 8. u
  if styles.underline then xml = xml .. '<w:u w:val="single"/>' end

  -- 9. shd
  if styles.bg_color then
    local c = styles.bg_color:gsub('#', '')
    xml = xml .. '<w:shd w:val="clear" w:color="auto" w:fill="' .. c .. '"/>'
  end

  -- 10. vertAlign
  if styles.vertAlign then xml = xml .. '<w:vertAlign w:val="' .. styles.vertAlign .. '"/>' end

  xml = xml .. '</w:rPr>'
  xml = xml .. '<w:t xml:space="preserve">' .. escape_xml(text) .. '</w:t>'
  xml = xml .. '</w:r>'

  return pandoc.RawInline('openxml', xml)
end

function shallow_copy(orig)
    local orig_type = type(orig)
    local copy
    if orig_type == 'table' then
        copy = {}
        for orig_key, orig_value in pairs(orig) do
            copy[orig_key] = orig_value
        end
    else -- number, string, boolean, etc
        copy = orig
    end
    return copy
end

function escape_xml(str)
  str = string.gsub(str, "&", "&amp;")
  str = string.gsub(str, "<", "&lt;")
  str = string.gsub(str, ">", "&gt;")
  str = string.gsub(str, "\"", "&quot;")
  str = string.gsub(str, "'", "&apos;")
  return str
end
