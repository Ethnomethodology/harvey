-- src-tauri/scripts/docx_styles.lua

-- Helper to split a string
function string:split(sep)
    local sep, fields = sep or ":", {}
    local pattern = string.format("([^%s]+)", sep)
    self:gsub(pattern, function(c) fields[#fields+1] = c end)
    return fields
end

-- Helper to trim a string
function string:trim()
    return self:match("^%s*(.-)%s*$")
end

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
      if el.attributes['data-color'] then 
          props.color = el.attributes['data-color'] 
      elseif el.attributes['color'] then
          props.color = el.attributes['color']
      end

      if el.attributes['data-font-family'] then 
          props.font = el.attributes['data-font-family'] 
      elseif el.attributes['font-family'] then
          props.font = el.attributes['font-family']
      end
      
      if el.attributes['data-font-size'] then 
          props.size = el.attributes['data-font-size'] 
      elseif el.attributes['font-size'] then
          props.size = el.attributes['font-size']
      end

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

-- Helper to map Lexical font stacks to standard Word font names
local function map_font_family(font)
    if not font then return nil end
    -- Remove quotes and take the first font in the stack
    local first = font:gsub('["\']', ''):split(',')[1]:trim()
    
    -- Mapping for Harvey's specific font options
    if first == "Inter" then return "Inter" end
    if first == "Roboto" then return "Roboto" end
    if first == "Montserrat" then return "Montserrat" end
    if first == "Bangers" then return "Bangers" end
    if first == "Indie Flower" then return "Indie Flower" end
    -- Comic Neue might not be installed on many systems, Word will default it. 
    -- Comic Sans MS is the standard Word alternative if Comic Neue is missing.
    if first == "Comic Neue" then return "Comic Neue" end
    if first == "Palatino Linotype" or first == "Palatino" or first == "Book Antiqua" then return "Palatino Linotype" end
    if first == "Times New Roman" or first == "Times" then return "Times New Roman" end
    if first == "Calibri" or first == "Candara" or first == "Segoe UI" then return "Calibri" end
    if first == "Comic Sans MS" or first == "Comic Sans" then return "Comic Sans MS" end
    if first == "Arial" or first == "Helvetica" then return "Arial" end
    if first == "Courier New" or first == "Courier" then return "Courier New" end
    if first == "Courier Prime" then return "Courier Prime" end
    if first == "Merriweather" then return "Merriweather" end
    -- Consolas is a high-quality modern monospaced font available on Windows and commonly on Mac via Office.
    if first == "Monaco" or first == "Consolas" or first == "Lucida Console" then return "Consolas" end
    
    return first
end

-- Helper to generate the <w:rPr> string based on properties
local function generate_rpr(props)
  local rPr = ""

  -- Order is important in OpenXML ECMA-376 (Strict)
  -- Reference: http://officeopenxml.com/WPtextFormatting.php
  -- Order: rStyle, rFonts, b, i, strike, color, sz, highlight, u, effect, bdr, shd

  -- 0. rStyle (Hyperlink)
  if props.is_link then
      rPr = rPr .. '<w:rStyle w:val="Hyperlink"/>'
  end

  -- 1. rFonts
  if props.font then
     local mapped_font = map_font_family(props.font)
     if mapped_font then
        local f = escape_xml(mapped_font)
        rPr = rPr .. string.format('<w:rFonts w:ascii="%s" w:hAnsi="%s" w:cs="%s"/>', f, f, f)
     end
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
  if props.underline then
      local u_color = ""
      if props.color then
          local c = props.color:gsub("#", "")
          u_color = string.format(' w:color="%s"', c)
      end
      rPr = rPr .. string.format('<w:u w:val="single"%s/>', u_color)
  end

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

    elseif elem.t == 'Link' then
        -- Handle links by recursing into content with link property set
        local sub_props = clone(props)
        sub_props.is_link = true

        -- Recurse into link content
        local sub_res = collect_text(elem.content, sub_props)

        -- Instead of returning a pandoc.Link (which lets Pandoc control the OpenXML),
        -- we manually construct the <w:hyperlink> element to ensure our styles persist.
        -- This requires a unique ID (r:id) for the relationship, but in a filter we cannot easily register new relationships.
        -- HOWEVER, Pandoc allows raw OpenXML.
        -- A robust fallback is to rely on Pandoc's Link handling but force the run properties INSIDE the link text runs.
        -- The previous attempt failed because Word's "Hyperlink" character style overrides direct formatting if not carefully applied,
        -- OR Pandoc strips the direct formatting when wrapping in a Link.

        -- Strategy B: Use pandoc.Link but try to inject a RawInline that forces the style?
        -- No, let's try to wrap the inner runs in a Span that Pandoc might respect?
        -- No, let's stick to returning the Link with styled content, but we need to ensure collect_text returns RawInlines that Word respects.
        -- The issue is likely that <w:hyperlink> in Word applies the "Hyperlink" style which is blue/underlined by default in clean templates,
        -- but if the template differs, it might not be.
        -- If we want to FORCE it, we must ensure the <w:rPr> inside the hyperlink's <w:r> has <w:color> and <w:u>.
        -- Our collect_text DOES generate <w:r><w:rPr>... so the issue might be Pandoc wrapping.

        -- Strategy C: Construct a raw OpenXML hyperlink if possible.
        -- Since we can't easily generate the relationship ID without access to the document relationships part,
        -- we must rely on Pandoc to generate the relationship.
        -- But we can try to wrap the content in a way that preserves style.

        -- Let's try returning the Link, but double-check that sub_res are indeed RawInlines.
        -- They are.

        -- Maybe we need to explicitely set the style of the runs to NOT be "Hyperlink" so they don't get overridden?
        -- Or conversely, maybe we need to ensure they HAVE the color/underline manually.
        -- The user says it IS clickable (so it is a link) but NOT blue/underlined.
        -- This implies the runs inside the hyperlink lack the formatting.

        -- Let's try a different approach: Return the Link, but make sure the props passed to collect_text
        -- are definitely applied. They are.

        -- Wait, if Pandoc sees RawInlines inside a Link, does it wrap them in ANOTHER run?
        -- If `sub_res` is a list of RawInline('openxml', ...), Pandoc should just output them inside the hyperlink.
        -- If so, the XML structure should be <w:hyperlink ...> <w:r>...</w:r> </w:hyperlink>.
        -- And our <w:r> has <w:rPr><w:color .../><w:u .../></w:rPr>.
        -- So why does it not show?

        -- Possibility: The default Hyperlink style in Word has precedence or conflicts.
        -- Or maybe the user's Word settings / template.
        -- Let's explicitly Add <w:rStyle w:val="Hyperlink"/> AND the direct formatting.
        -- Actually, if we want to FORCE blue/underline regardless of theme, we are doing the right thing.

        -- Let's try to ensure we are using the correct color format. "0000FF" is correct.

        -- RE-READING: "Link texts are not appearing in blue and underlined... but its clickable".
        -- This means the <w:hyperlink> tag is there.
        -- Maybe the runs inside are being stripped?

        -- Let's try one trick: Insert a zero-width space with the style? No.

        -- Let's try to return the Link object but use a specific class/attribute that our filter acts on?
        -- No, we are IN the filter.

        -- Let's just return the Link with the processed content.
        -- But let's verify if `collect_text` is actually called for Link.
        -- Yes, because we added the Link case.

        -- If the previous plan failed, it implies `pandoc.Link` might be ignoring the RawInlines or re-processing them.
        -- Let's try to modify the top-level Link function to ensure it's catching them.
        -- It is.

        -- Let's try a different tack: The content of a Link in Pandoc is a list of Inlines.
        -- `collect_text` returns a list of RawInlines.
        -- When Pandoc writes this to Docx, it puts the RawXML inside the hyperlink.
        -- If that RawXML is `<w:r>...</w:r>`, it should work.

        -- Maybe we need to explicitly clear the rStyle? <w:rStyle w:val=""/> isn't valid.
        -- But we can try to explicitly set the rStyle to something benign or standard?

        -- CRITICAL: `pandoc.Link` in a Lua filter might be converting its content back to standard inlines if not careful?
        -- No, RawInline should be preserved.

        -- Let's look at `collect_text` again.
        -- It generates `<w:r><w:rPr>...</w:rPr><w:t>...</w:t></w:r>`.
        -- If this is inside a `<w:hyperlink>`, it is valid OpenXML.

        -- What if we explicitly add `w:val` to underline? `w:val="single"`. We are doing that.

        -- Let's try to force the styling by wrapping the content in a distinct run property
        -- that Pandoc *can't* mess with? No, RawInline is the ultimate force.

        -- HYPOTHESIS: Pandoc's docx writer might be wrapping the link content in its OWN run if it detects simple text,
        -- effectively ignoring our RawInline if it can, or maybe wrapping our RawInline in another run?
        -- No, RawInline 'openxml' is usually dumped as-is.

        -- Let's try to use `pandoc.utils.stringify` to verify content? No.

        -- Let's assume the previous code WAS working but maybe cached/not applied?
        -- Or maybe the user's viewer (LibreOffice? Word?) behaves differently.

        -- Let's try to make the Link content purely RawInline.
        -- We are doing that.

        -- ALTERNATIVE: Don't use `pandoc.Link`. Use `pandoc.RawInline` to write the field code for a hyperlink.
        -- Word supports fields for hyperlinks: { HYPERLINK "url" }.
        -- This avoids the Relationship ID issue!
        -- Syntax: <w:fldSimple w:instr=" HYPERLINK &quot;url&quot; "> ... runs ... </w:fldSimple>

        local url = escape_xml(elem.target)
        local sub_props = clone(props)
        sub_props.is_link = true

        -- Recurse into link content to get styled runs
        local sub_res = collect_text(elem.content, sub_props)

        -- Construct fldSimple XML
        -- Note: fldSimple is robust and doesn't require rId.
        local runs_xml = ""
        for _, inline in ipairs(sub_res) do
            if inline.t == 'RawInline' then
                runs_xml = runs_xml .. inline.text
            else
                -- Should not happen with collect_text but for safety
                runs_xml = runs_xml .. pandoc.utils.stringify(inline)
            end
        end

        local xml = string.format('<w:fldSimple w:instr=" HYPERLINK &quot;%s&quot; ">%s</w:fldSimple>', url, runs_xml)
        table.insert(result, pandoc.RawInline('openxml', xml))

    else
       -- Fallback for other elements
       table.insert(result, elem)
    end
  end
  return result
end

function Link(el)
    -- Handle top-level links
    local props = {
        is_link = true
    }

    -- We use the native pandoc.Link but ensure the content has the "Hyperlink" style
    -- by wrapping the styled runs in a Span with custom-style="Hyperlink".
    -- This triggers Pandoc's native rStyle mapping.

    -- Get the manually styled runs (OpenXML)
    local sub_res = collect_text(el.content, props)

    -- Wrap them in a Span that Pandoc understands as a style trigger
    local styled_content = pandoc.Span(sub_res, {['custom-style'] = 'Hyperlink'})

    return pandoc.Link(styled_content, el.target, el.title, el.attr)
end

function CodeBlock(el)
    local code = el.text
    local lines = code:split("\n")
    -- Prefix and suffix newlines to ensure block separation in XML
    local result_xml = "\n"

    local num_lines = #lines

    for i, line in ipairs(lines) do
        local safe_line = escape_xml(line)

        -- Determine paragraph borders based on line position
        -- This connects them into a single cohesive box visually in Word.
        local top_border = ""
        local bottom_border = ""

        if i == 1 then
            top_border = '<w:top w:val="single" w:sz="4" w:space="5" w:color="auto"/>'
        end
        if i == num_lines then
            bottom_border = '<w:bottom w:val="single" w:sz="4" w:space="5" w:color="auto"/>'
        end

        local borders_xml = string.format(
            '<w:pBdr>' ..
              '%s' ..
              '<w:left w:val="single" w:sz="4" w:space="5" w:color="auto"/>' ..
              '%s' ..
              '<w:right w:val="single" w:sz="4" w:space="5" w:color="auto"/>' ..
            '</w:pBdr>',
            top_border, bottom_border
        )

        local p = string.format(
            '<w:p>' ..
              '<w:pPr>' ..
                '<w:pStyle w:val="NoSpacing"/>' ..
                '<w:shd w:val="clear" w:color="auto" w:fill="EAEAEA"/>' ..
                '%s' ..
                '<w:ind w:left="120" w:right="120"/>' ..
                '<w:spacing w:after="0" w:line="240" w:lineRule="auto"/>' ..
              '</w:pPr>' ..
              '<w:r>' ..
                '<w:rPr>' ..
                  '<w:shd w:val="clear" w:color="auto" w:fill="EAEAEA"/>' ..
                  '<w:rFonts w:ascii="Courier Prime" w:hAnsi="Courier Prime" w:cs="Courier Prime"/>' ..
                  '<w:sz w:val="19"/>' ..
                  '<w:szCs w:val="19"/>' ..
                '</w:rPr>' ..
                '<w:t xml:space="preserve">%s</w:t>' ..
              '</w:r>' ..
            '</w:p>',
            borders_xml, safe_line
        )
        result_xml = result_xml .. p
    end

    result_xml = result_xml .. '<w:p><w:pPr><w:spacing w:after="120"/></w:pPr></w:p>\n'

    return pandoc.RawBlock('openxml', result_xml)
end

function BlockQuote(el)
    local result_xml = "\n"

    for _, block in ipairs(el.content) do
        if block.t == 'Para' or block.t == 'Plain' then
            local props = { italic = true }
            local runs = collect_text(block.content, props)
            local runs_xml = ""
            for _, run in ipairs(runs) do
                if run.t == 'RawInline' then runs_xml = runs_xml .. run.text end
            end

            local p = string.format(
                '<w:p>' ..
                  '<w:pPr>' ..
                    '<w:pStyle w:val="Quote"/>' ..
                  '</w:pPr>' ..
                  '%s' ..
                '</w:p>',
                runs_xml
            )
            result_xml = result_xml .. p
        end
    end
    result_xml = result_xml .. "\n"
    return pandoc.RawBlock('openxml', result_xml)
end

function Span(el)
  if el.classes:includes('custom-math') then
    local math_text = pandoc.utils.stringify(el)
    local is_inline = el.attributes['data-inline'] == 'true'
    local math_type = is_inline and pandoc.InlineMath or pandoc.DisplayMath
    return pandoc.Math(math_type, math_text)
  end

  -- Only process Spans that have our target attributes
  local color = el.attributes['data-color'] or el.attributes['color']
  local font = el.attributes['data-font-family'] or el.attributes['font-family']
  local size = el.attributes['data-font-size'] or el.attributes['font-size']
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

function Table(el)
    -- Check for an existing custom-style or a class that might represent a style name.
    -- This allows specifically styled tables (like "code_block") to persist.
    local style = el.attributes['custom-style'] or (el.attr and el.attr.attributes['custom-style'])

    if not style or style == "" then
        -- Fallback to the first class if one exists and isn't a known layout class
        if el.classes and #el.classes > 0 then
            style = el.classes[1]
            -- If the class matches what we know we want for code blocks, use it as the style.
            if style == "codeblock" or style == "code_block" then
                style = "codeblock"
            elseif style == "Table" or style == "TableGrid" or style == "Table Grid" then
                style = "Table"
            end
        else
            style = "Table"
        end
    end

    if el.classes then
        el.classes:insert((style:gsub("%s+", ""))) -- Remove spaces for class ID, extra () ensures one return value
        el.attributes['custom-style'] = style
        el.attributes['border'] = '1'
    elseif el.attr then
        el.attr.classes:insert((style:gsub("%s+", "")))
        el.attr.attributes['custom-style'] = style
        el.attr.attributes['border'] = '1'
    end
    return el
end
