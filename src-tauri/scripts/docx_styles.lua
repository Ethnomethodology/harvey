function Span(el)
  -- Parse styles from data- attributes which we will populate in Rust
  -- Or strictly from style attribute if we stick to one approach.
  -- But standard HTML reader strips many styles.
  -- We will use data- attributes for reliable parsing.

  local color = el.attributes['data-color']
  local bg_color = el.attributes['data-bg-color']
  local font_family = el.attributes['data-font-family']
  local font_size = el.attributes['data-font-size']

  if not (color or bg_color or font_family or font_size) then
    return nil
  end

  -- Start building OpenXML Run Properties
  local openxml = '<w:rPr>'

  if color then
    -- Remove # if present
    color = color:gsub('#', '')
    openxml = openxml .. '<w:color w:val="' .. color .. '"/>'
  end

  if bg_color then
    bg_color = bg_color:gsub('#', '')
    -- w:shd is more flexible than w:highlight (which is limited to specific colors)
    openxml = openxml .. '<w:shd w:val="clear" w:color="auto" w:fill="' .. bg_color .. '"/>'
  end

  if font_family then
    openxml = openxml .. '<w:rFonts w:ascii="' .. font_family .. '" w:hAnsi="' .. font_family .. '"/>'
  end

  if font_size then
    -- Convert pts to half-points (1/144 inch)
    -- Assuming input is like "14pt" or "14"
    local size_val = font_size:gsub('pt', ''):gsub('px', '') -- simple cleanup
    local size_num = tonumber(size_val)
    if size_num then
        local half_points = math.floor(size_num * 2)
        openxml = openxml .. '<w:sz w:val="' .. half_points .. '"/>'
        openxml = openxml .. '<w:szCs w:val="' .. half_points .. '"/>'
    end
  end

  openxml = openxml .. '</w:rPr>'

  -- Wrap content in a RawInline with OpenXML
  -- We need to ensure the inner content is processed by Pandoc first?
  -- No, injecting Raw OpenXML effectively replaces standard rendering for this span's container.
  -- But we want to preserve nested formatting (bold, italic) which are standard.
  --
  -- Pandoc's docx writer is tricky with RawBlock/RawInline.
  -- Best approach for modifying properties of an existing run is actually Custom Styles,
  -- but those are paragraph-level or character styles defined in reference doc.
  --
  -- To apply arbitrary inline formatting without defined styles, we must emit Raw OpenXML.
  -- BUT, if we emit <w:r>, we must also emit the text content inside <w:t>.
  -- And we must recursively process the children (el.content).

  local inner_xml = ""
  for _, item in ipairs(el.content) do
    if item.t == 'Str' then
      inner_xml = inner_xml .. '<w:t xml:space="preserve">' .. escape_xml(item.text) .. '</w:t>'
    elseif item.t == 'Space' then
      inner_xml = inner_xml .. '<w:t xml:space="preserve"> </w:t>'
    elseif item.t == 'Strong' then
       -- This gets complicated fast. Nested Bold inside our color span.
       -- A pure Lua filter outputting RawXML has to handle ALL nesting manually.
       -- Alternatively, can we just set attributes that Pandoc natively understands?
       -- No, Pandoc ignores color/font.
    else
       -- Fallback for simple text
       if item.text then
         inner_xml = inner_xml .. '<w:t>' .. escape_xml(item.text) .. '</w:t>'
       end
    end
  end

  -- Since reimplementing full DOCX serialization for nested nodes (Bold, Italic) in Lua is hard,
  -- A better strategy:
  -- Output the <w:rPr> ... </w:rPr> at the start of the run?
  -- No, in DOCX, properties must be inside the <w:r>.

  -- SIMPLIFIED STRATEGY for this task:
  -- The user states Bold/Italic/Underline work.
  -- We are adding Color/Font/Size.
  -- We can output a RawInline that opens a group with properties? No, DOCX doesn't stack like HTML.

  -- Workaround:
  -- We will rely on the fact that these are leaf nodes (Text) in our specific Lexical conversion logic.
  -- In `export_handler.rs`, we handle Bold/Italic/Underline by generating <b>, <i> tags.
  -- If we generate <span data-color="..."><b>Text</b></span>, Pandoc sees Span > Strong > Str.

  -- If we use a Lua filter, we can walk the tree.
  -- If we find a Span with data-color, we can traverse its children.
  -- For every String (Str) inside, we convert it to a RawInline('openxml', ...)
  -- that contains the <w:r> <w:rPr> [our colors] [bold/italic from context?] <w:t>text</w:t> </w:r>

  -- Detecting Bold/Italic context in Lua filter:
  -- We can write a recursive function that carries formatting state.

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

  local xml = '<w:r>'
  xml = xml .. '<w:rPr>'

  if styles.bold then xml = xml .. '<w:b/>' end
  if styles.italic then xml = xml .. '<w:i/>' end
  if styles.underline then xml = xml .. '<w:u w:val="single"/>' end
  if styles.strike then xml = xml .. '<w:strike/>' end
  if styles.vertAlign then xml = xml .. '<w:vertAlign w:val="' .. styles.vertAlign .. '"/>' end

  if styles.color then
    local c = styles.color:gsub('#', '')
    xml = xml .. '<w:color w:val="' .. c .. '"/>'
  end
  if styles.bg_color then
    local c = styles.bg_color:gsub('#', '')
    -- Force w:shd for everything to ensure custom colors work.
    -- w:highlight is too restrictive (only supports ~15 presets).
    -- Word renders w:shd as "Character Shading" which looks like highlight.
    xml = xml .. '<w:shd w:val="clear" w:color="auto" w:fill="' .. c .. '"/>'
  end
  if styles.font_family then
    -- Set ALL font slots to the target font to force override.
    -- w:eastAsia is important for some versions of Word/locales.
    xml = xml .. '<w:rFonts w:ascii="' .. styles.font_family .. '" w:hAnsi="' .. styles.font_family .. '" w:cs="' .. styles.font_family .. '" w:eastAsia="' .. styles.font_family .. '"/>'
  end
  if styles.font_size then
    local size_val = styles.font_size:gsub('pt', ''):gsub('px', '')
    local size_num = tonumber(size_val)
    if size_num then
        local half_points = math.floor(size_num * 2)
        xml = xml .. '<w:sz w:val="' .. half_points .. '"/>'
        xml = xml .. '<w:szCs w:val="' .. half_points .. '"/>'
    end
  end

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
