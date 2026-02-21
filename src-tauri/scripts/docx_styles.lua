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

-- OpenXML Standard Highlight Colors (Hex to Name Map)
-- We only support these standard values for <w:highlight w:val="..."/>
-- Anything else might fail in Compatibility Mode if we use w:shd.
-- So we map everything to the nearest standard color or just yellow.
local highlight_map = {
  ["FFFF00"] = "yellow",
  ["00FF00"] = "green",
  ["00FFFF"] = "cyan",
  ["FF00FF"] = "magenta",
  ["0000FF"] = "blue",
  ["FF0000"] = "red",
  ["000080"] = "darkBlue",
  ["008080"] = "darkCyan",
  ["008000"] = "darkGreen",
  ["800080"] = "darkMagenta",
  ["800000"] = "darkRed",
  ["808000"] = "darkYellow",
  ["808080"] = "darkGray",
  ["C0C0C0"] = "lightGray",
  ["000000"] = "black"
  -- white is not typically a highlight "color" in this sense, usually "none"
}

-- Simple color distance function to find nearest standard color
-- Returns the name of the nearest color
local function get_nearest_highlight(hex)
    local r = tonumber(hex:sub(1, 2), 16) or 0
    local g = tonumber(hex:sub(3, 4), 16) or 0
    local b = tonumber(hex:sub(5, 6), 16) or 0

    local min_dist = 999999999
    local best_match = "yellow" -- Default fallback

    for k, v in pairs(highlight_map) do
        local kr = tonumber(k:sub(1, 2), 16) or 0
        local kg = tonumber(k:sub(3, 4), 16) or 0
        local kb = tonumber(k:sub(5, 6), 16) or 0

        local dist = (r - kr)^2 + (g - kg)^2 + (b - kb)^2
        if dist < min_dist then
            min_dist = dist
            best_match = v
        end
    end

    return best_match
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
     -- We explicitly clear theme attributes to force the font to apply
     rPr = rPr .. string.format('<w:rFonts w:ascii="%s" w:hAnsi="%s" w:cs="%s" w:asciiTheme="" w:hAnsiTheme="" w:cstheme=""/>', f, f, f)
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

  -- 7. Highlight (Standard Colors ONLY) - Before Underline
  if props.highlight then
     local h = props.highlight:gsub("#", ""):upper()
     local highlight_val = "yellow" -- Default

     if highlight_map[h] then
         highlight_val = highlight_map[h]
     else
         -- Find nearest standard color
         highlight_val = get_nearest_highlight(h)
     end

     rPr = rPr .. string.format('<w:highlight w:val="%s"/>', highlight_val)
  end

  -- 8. Underline
  if props.underline then rPr = rPr .. '<w:u w:val="single"/>' end

  -- 9. Shading (Removed to fix compatibility issues)
  -- We rely entirely on <w:highlight> now.

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
