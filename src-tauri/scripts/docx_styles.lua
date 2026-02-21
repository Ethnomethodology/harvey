-- src-tauri/scripts/docx_styles.lua

-- Helper to escape XML characters
local function escape_xml(s)
  if s == nil then return "" end
  return s:gsub("&", "&amp;")
          :gsub("<", "&lt;")
          :gsub(">", "&gt;")
          :gsub('"', "&quot;")
          :gsub("'", "&apos;")
end

-- Helper to clone a table (shallow copy)
local function clone(t)
  local new_t = {}
  for k, v in pairs(t) do new_t[k] = v end
  return new_t
end

-- Helper to merge properties from a Span element into current props
local function merge_props(current, el)
  local props = clone(current)

  if el.t == 'Span' then
      if el.attributes['data-color'] then props.color = el.attributes['data-color'] end
      if el.attributes['data-font-family'] then props.font = el.attributes['data-font-family'] end
      if el.attributes['data-font-size'] then props.size = el.attributes['data-font-size'] end
      -- Pandoc might strip 'data-' prefix from 'data-highlight' in some versions or if using specific readers
      if el.attributes['data-highlight'] then 
          props.highlight = el.attributes['data-highlight'] 
      elseif el.attributes['highlight'] then
          props.highlight = el.attributes['highlight']
      end
  elseif el.t == 'Mark' then
      props.highlight = "yellow"
  end

  return props
end

-- Helper to map hex colors to Word's limited set of highlight colors
local function map_highlight_color(hex)
    if not hex then return nil end
    hex = hex:lower():gsub("#", "")
    
    -- Exact matches for common Harvey colors or defaults
    if hex == "ffff00" or hex == "yellow" then return "yellow" end
    if hex == "00ff00" or hex == "green" or hex == "lime" then return "green" end
    if hex == "00ffff" or hex == "cyan" or hex == "aqua" then return "cyan" end
    if hex == "ff00ff" or hex == "magenta" or hex == "fuchsia" then return "magenta" end
    if hex == "0000ff" or hex == "blue" then return "blue" end
    if hex == "ff0000" or hex == "red" then return "red" end
    if hex == "000080" or hex == "darkblue" then return "darkBlue" end
    if hex == "008080" or hex == "darkcyan" or hex == "teal" then return "darkCyan" end
    if hex == "008000" or hex == "darkgreen" then return "darkGreen" end
    if hex == "800080" or hex == "darkmagenta" or hex == "purple" then return "darkMagenta" end
    if hex == "800000" or hex == "darkred" or hex == "maroon" then return "darkRed" end
    if hex == "808000" or hex == "darkyellow" or hex == "olive" then return "darkYellow" end
    if hex == "808080" or hex == "gray" or hex == "grey" then return "lightGray" end
    if hex == "c0c0c0" or hex == "lightgray" then return "lightGray" end
    if hex == "000000" or hex == "black" then return "black" end
    
    -- Fallback to yellow for any highlight if we can't map it
    return "yellow"
end

-- Helper to generate the <w:rPr> string based on properties
local function generate_rpr(props)
  local rPr = ""

  -- Order is important in OpenXML ECMA-376 (Strict)
  -- Reference: http://officeopenxml.com/WPtextFormatting.php
  -- Order: rFonts, b, i, strike, color, sz, highlight, u, effect, bdr, shd

  -- 1. rFonts
  if props.font then
     local f = escape_xml(props.font)
     rPr = rPr .. string.format('<w:rFonts w:ascii="%s" w:hAnsi="%s" w:cs="%s"/>', f, f, f)
  end

  -- 2. Bold
  if props.bold then rPr = rPr .. '<w:b/>' end

  -- 3. Italic
  if props.italic then rPr = rPr .. '<w:i/>' end

  -- 4. Strike
  if props.strike then rPr = rPr .. '<w:strike/>' end

  -- 5. Color
  if props.color then
     local c = props.color:gsub("#", "")
     rPr = rPr .. string.format('<w:color w:val="%s"/>', c)
  end

  -- 6. Size
  if props.size then
     local n_str = props.size:match("[%d%.]+")
     if n_str then
         local pt = tonumber(n_str)
         if props.size:find("px") then pt = pt * 0.75 end
         local half_pts = math.floor(pt * 2)
         rPr = rPr .. string.format('<w:sz w:val="%d"/><w:szCs w:val="%d"/>', half_pts, half_pts)
     end
  end

  -- 7. Highlight
  if props.highlight then
     local word_color = map_highlight_color(props.highlight)
     if word_color then
        rPr = rPr .. string.format('<w:highlight w:val="%s"/>', word_color)
     end
  end

  -- 8. Underline
  if props.underline then rPr = rPr .. '<w:u w:val="single"/>' end

  return rPr
end

-- Recursive function to walk inline elements and apply properties
local function collect_text(inlines, props)
  local result = {}

  for _, elem in ipairs(inlines) do
    if elem.t == 'Str' then
      local text = escape_xml(elem.text)
      local rPr = generate_rpr(props)
      local xml = string.format('<w:r><w:rPr>%s</w:rPr><w:t xml:space="preserve">%s</w:t></w:r>', rPr, text)
      table.insert(result, pandoc.RawInline('openxml', xml))

    elseif elem.t == 'Space' then
       local rPr = generate_rpr(props)
       local xml = string.format('<w:r><w:rPr>%s</w:rPr><w:t xml:space="preserve"> </w:t></w:r>', rPr)
       table.insert(result, pandoc.RawInline('openxml', xml))

    elseif elem.t == 'SoftBreak' or elem.t == 'LineBreak' then
       local rPr = generate_rpr(props)
       local xml = string.format('<w:r><w:rPr>%s</w:rPr><w:br/></w:r>', rPr)
       table.insert(result, pandoc.RawInline('openxml', xml))

    elseif elem.t == 'Strong' then
      local sub_props = clone(props)
      sub_props.bold = true
      local sub_res = collect_text(elem.content, sub_props)
      for _, v in ipairs(sub_res) do table.insert(result, v) end

    elseif elem.t == 'Emph' then
      local sub_props = clone(props)
      sub_props.italic = true
      local sub_res = collect_text(elem.content, sub_props)
      for _, v in ipairs(sub_res) do table.insert(result, v) end

    elseif elem.t == 'Underline' then
       local sub_props = clone(props)
       sub_props.underline = true
       local sub_res = collect_text(elem.content, sub_props)
       for _, v in ipairs(sub_res) do table.insert(result, v) end

    elseif elem.t == 'Strikeout' then
       local sub_props = clone(props)
       sub_props.strike = true
       local sub_res = collect_text(elem.content, sub_props)
       for _, v in ipairs(sub_res) do table.insert(result, v) end

    elseif elem.t == 'Span' then
       -- Recursively handle nested Spans (e.g. <span highlight><span color>text</span></span>)
       -- Merge the new properties with the inherited ones
       local sub_props = merge_props(props, elem)
       local sub_res = collect_text(elem.content, sub_props)
       for _, v in ipairs(sub_res) do table.insert(result, v) end

    elseif elem.t == 'Mark' then
       -- Handle Mark (e.g. standard HTML <mark>)
       local sub_props = clone(props)
       sub_props.highlight = "yellow"
       local sub_res = collect_text(elem.content, sub_props)
       for _, v in ipairs(sub_res) do table.insert(result, v) end

    elseif elem.t == 'Subscript' or elem.t == 'Superscript' then
        -- Pass through unsupported elements as-is
        table.insert(result, elem)

    else
       -- Fallback for other elements
       table.insert(result, elem)
    end
  end
  return result
end

function Span(el)
  -- Only process Spans that have our target attributes
  local color = el.attributes['data-color']
  local font = el.attributes['data-font-family']
  local size = el.attributes['data-font-size']
  local highlight = el.attributes['data-highlight'] or el.attributes['highlight']

  if color or font or size or highlight then
    local props = {
      color = color,
      font = font,
      size = size,
      highlight = highlight
    }

    return collect_text(el.content, props)
  end
  -- Return nil to leave other spans untouched
end

function Mark(el)
    -- Handle top-level Mark elements that might be generated by Pandoc before Span filter runs
    local props = {
        highlight = "yellow"
    }
    return collect_text(el.content, props)
end
