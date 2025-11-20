local sorters = require("telescope.sorters")

local M = {}

function M.post_sorter(opts)
  opts = opts or {}
  -- We can use `fzy_sorter` for the actual fuzzy matching.
  local fzy_sorter = sorters.get_fzy_sorter(opts)

  return sorters.Sorter:new({
    -- Allow us to filter entries as well as sorting them.
    discard = false,

    scoring_function = function(_, prompt, entry)
      -- This mimics a standard fuzzy sorting on the entry title.
	  if type(entry) == "string" then
		return fzy_sorter:scoring_function(prompt, entry)
	  else
		if entry.context ~= nil and entry.context ~= vim.NIL then
		  return fzy_sorter:scoring_function(prompt, entry.context[1])
		end

		return fzy_sorter:scoring_function(prompt, entry.header or entry.msg)
	  end
    end,

    -- We could also specify a highlighter. The highlighter works fine in this case,
    -- but if we modify `scoring_function` we have to modify this too.
    -- I admit, I currently don't use a highlighter for my posts finder.
    highlighter = fzy_sorter.highlighter,
  })
end

function M.view_to_string(tabb)
  for k in pairs(tabb) do
	local v = tabb[k]
	if k == "anchor" then
	  local s = v:gsub("%.", " ")
	  s = s:gsub("^%l", string.upper)
	  return s
	elseif k == "search" then
	  return v
	elseif k == "page" then
	  return "p. " .. v
	else
	  return v
	end
  end
end

function M.getCenteredSubstring(str, pattern, maxChars)
	local lowerStr = string.lower(str)
    local lowerPattern = string.lower(pattern)
    -- Find the start and end positions of the pattern in the string
    local startPos, endPos = string.find(lowerStr, lowerPattern)
    if not startPos then
        return str
    end

    -- Calculate the half-length around the pattern
    local halfLength = math.floor((maxChars - (endPos - startPos + 1)) / 2)

    -- Calculate the start and end positions of the substring
    local subStart = math.max(1, startPos - halfLength)
    local subEnd = math.min(#str, endPos + halfLength)

    -- Adjust if the substring is too long (should not happen, but just in case)
    if subEnd - subStart + 1 > maxChars then
        subStart = subEnd - maxChars + 1
    end

    -- Return the substring
    return string.sub(str, subStart, subEnd)
end

return M
