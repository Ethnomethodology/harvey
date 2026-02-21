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

-- Helper to generate the <w:rPr> string based on properties
local function generate_rpr(props)
  local rPr = ""

  -- Order is important in OpenXML ECMA-376 (Strict)
  -- Reference: http://officeopenxml.com/WPtextFormatting.php
  -- Order: rFonts, b, i, strike, color, sz, highlight, u, effect, bdr, shd

  -- 1. rFonts
  if props.font then
     local f = escape_xml(props.font)
     -- Simplified to just ascii/hAnsi/cs to avoid invalid attributes
     rPr = rPr .. string.format('<w:rFonts w:ascii="%s" w:hAnsi="%s" w:cs="%s"/>', f, f, f)
  end

  -- 2. Bold
  if props.bold then rPr = rPr .. '<w:b/>' end

  -- 3. Italic
  if props.italic then rPr = rPr .. '<w:i/>' end

  -- 4. Strike (Must be before color)
  if props.strike then rPr = rPr .. '<w:strike/>' end

  -- 5. Color
  if props.color then
     local c = props.color:gsub("#", "")
     rPr = rPr .. string.format('<w:color w:val="%s"/>', c)
  end

  -- 6. Size
  if props.size then
     -- Extract number
     local n_str = props.size:match("[%d%.]+")
     if n_str then
         local pt = tonumber(n_str)
         -- Heuristic: if unit is px, convert to pt (approx 0.75)
         if props.size:find("px") then
             pt = pt * 0.75
         end
         local half_pts = math.floor(pt * 2)
         rPr = rPr .. string.format('<w:sz w:val="%d"/><w:szCs w:val="%d"/>', half_pts, half_pts)
     end
  end

  -- 7. Highlight (Simplified: Always Yellow if present)
  if props.highlight then
     -- User explicitly requested fallback to yellow for ALL highlights
     rPr = rPr .. '<w:highlight w:val="yellow"/>'
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
      -- Wrap text in a Run with explicit properties
      local xml = string.format('<w:r><w:rPr>%s</w:rPr><w:t xml:space="preserve">%s</w:t></w:r>', rPr, text)
      table.insert(result, pandoc.RawInline('openxml', xml))

    elseif elem.t == 'Space' then
       local rPr = generate_rpr(props)
       local xml = string.format('<w:r><w:rPr>%s</w:rPr><w:t xml:space="preserve"> </w:t></w:r>', rPr)
       table.insert(result, pandoc.RawInline('openxml', xml))

    elseif elem.t == 'SoftBreak' then
       local rPr = generate_rpr(props)
       -- Use w:br inside run
       local xml = string.format('<w:r><w:rPr>%s</w:rPr><w:br/></w:r>', rPr)
       table.insert(result, pandoc.RawInline('openxml', xml))

    elseif elem.t == 'LineBreak' then
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

    elseif elem.t == 'Subscript' then
        table.insert(result, elem)
    elseif elem.t == 'Superscript' then
        table.insert(result, elem)

    else
       -- Fallback for elements we don't want to break
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
  local highlight = el.attributes['data-highlight']

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
