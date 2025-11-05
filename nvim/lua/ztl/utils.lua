local fwatch = require('fwatch')
local toml = require("toml")
local log = require "ztl.log"

local M = {}

local watcher = nil
function M.call_and_watch(fdir, fnc, jitter)
  if watcher ~= nil then
	  fwatch.unwatch(watcher)
  end

  local path = fnc()

  local defer = nil
  watcher = fwatch.watch(path, {on_event = function()
	if not defer then -- only set once in window
	  defer = vim.defer_fn(function()
		defer = nil -- clear for next event out side of window
		fnc() -- do work
	  end, jitter)  -- run in 100 ms, probably this can be much lower.
	end
  end})
end

function M.toml(fname)
  succeeded, notes = pcall(toml.decodeFromFile, fname)
  
  if not succeeded then
	vim.notify("Could not parse " .. fname .. "\n\n => " .. notes["reason"], "error")
	return nil
  end

  return notes
end

local sets = {{97, 122}, {48, 57}} -- a-z, 0-9
function M.string_random(chars)
	local str = ""
	for i = 1, chars do
		math.randomseed(os.clock() ^ 5)
		local charset = sets[ math.random(1, #sets) ]
		str = str .. string.char(math.random(charset[1], charset[2]))
	end
	return str
end

function M.log(_, data)
    if data then
        -- Make it temporarily writable so we don't have warnings.
        vim.api.nvim_buf_set_option(buffer_number, "readonly", false)
        
        -- Append the data.
        vim.api.nvim_buf_set_lines(buffer_number, -1, -1, true, vim.split(data, "\n"))

        -- Make readonly again.
        vim.api.nvim_buf_set_option(buffer_number, "readonly", true)

        -- Mark as not modified, otherwise you'll get an error when
        -- attempting to exit vim.
        vim.api.nvim_buf_set_option(buffer_number, "modified", false)

        -- Get the window the buffer is in and set the cursor position to the bottom.
        local buffer_window = vim.api.nvim_call_function("bufwinid", { buffer_number })
        local buffer_line_count = vim.api.nvim_buf_line_count(buffer_number)
        vim.api.nvim_win_set_cursor(buffer_window, { buffer_line_count, 0 })
    end
end

function M.open_buffer()
    -- Get a boolean that tells us if the buffer number is visible anymore.
    --
    -- :help bufwinnr
    local buffer_visible = vim.api.nvim_call_function("bufwinnr", { buffer_number }) ~= -1

    if buffer_number == -1 or not buffer_visible then
        -- Create a new buffer with the name "AUTOTEST_OUTPUT".
        -- Same name will reuse the current buffer.
        vim.api.nvim_command("botright split llm-response")
        
        -- Collect the buffer's number.
        buffer_number = vim.api.nvim_get_current_buf()
        
        -- Mark the buffer as readonly.
        vim.opt_local.readonly = true
    end
end

function M.open(span, target, view)
  local config = require("ztl.config").options

  -- split resource string into identifier and resource
  local identifier, resource = string.match(target,  "^([%a_][%w_]*)%s*:%s*([^\n]*)$")
  if config.resources[identifier] ~= nil then
	  resource = config.resources[identifier](resource)
  end

  -- if identifier points to note, open it
  log.info(target)
  if identifier == "key" then
	local note = M.toml(span:cache_dir() .. resource)
	M.open_file(note["span"]["source"], note["span"]["start"]["line"])
	return
  end

  if resource:sub(-4) == ".pdf" then
    cmd = config.viewer["pdf"](resource, view)
  elseif string.sub(resource, 1, 4) == "http" then
	cmd = config.viewer["website"](resource, view)
  elseif resource:sub(-3) == ".md" then
	cmd = config.viewer["markdown"](resource, view)
  else
	vim.notify("Unknown resource: " .. resource)
  end

  if cmd ~= nil then
	vim.fn.jobstart(cmd, {
	  on_error = function(err)
		vim.notify("Could not run command " .. cmd, "error")
	  end
	})
  end
end

function M.open_file(fname, line)
	function jump_to()
	  vim.api.nvim_win_set_cursor(0, {line, 0})

	  -- press all thumbs that this happens after loading folding 
	  vim.schedule(function() vim.cmd("normal! zO") end)
	end

	-- check if current open buffer has given fname
	local current_buf_name = vim.api.nvim_buf_get_name(0)
	if string.find(current_buf_name, fname, 1, true) then
		jump_to()
	else
		vim.api.nvim_create_autocmd('BufWinEnter', {
			pattern = fname,
			callback = jump_to,
			once = true
		})

		vim.cmd("edit " .. fname)
	end
end

-- perform regex on markdown path and open all matches in vertical 
-- splits; allows for parallel reading
function M.open_md_in_tabs(file) 
  local tabs = vim.api.nvim_list_tabpages()
  local tabpage = -1
  for i, tab in ipairs(tabs) do
	local wins = vim.api.nvim_tabpage_list_wins(tab)

	for j, win in ipairs(wins) do
	  local buf = vim.api.nvim_buf_get_name(vim.api.nvim_win_get_buf(win))

	  if file == buf then
		tabpage = vim.api.nvim_tabpage_get_number(tab)
	  end
	end
  end

  if tabpage == -1 then
	local pat = vim.fn.fnamemodify(file, ":r") .. "*.md"
	for i, file in ipairs(vim.fn.glob(pat, false, true)) do
	  if i == 1 then
		vim.cmd("tabnew " .. file)
	  else
		vim.cmd("bel vsp " .. file)
		vim.opt.tw = 60
	  end
	  vim.opt.scrollbind = true
	end
  else
	vim.api.nvim_set_current_tabpage(tabpage)
  end
end

function M.remove_prefix(full_path, prefix)
    -- Ensure both paths use the same separator (e.g., '/')
    full_path = full_path:gsub("\\", "/")
    prefix = prefix:gsub("\\", "/")

    -- Check if the full_path starts with the prefix
    if full_path:sub(1, #prefix) == prefix then
        -- Remove the prefix and any leading separator
        return full_path:sub(#prefix + 1)
    else
        return full_path -- or handle error as needed
    end
end

--- Truncate or pad a string to a fixed length, preserving the end.
--- If longer than max_len, the start is replaced with an ellipsis (…).
--- If shorter, it's padded with spaces to the right.
--- @param str string
--- @param max_len integer
--- @return string
function M.fit_to_width(str, max_len)
  local ellipsis = "…"
  if #str > max_len then
    -- truncate and keep end
    local keep = max_len - #ellipsis
    str = ellipsis .. string.sub(str, -keep)
  elseif #str < max_len then
    -- pad with spaces
    str = str .. string.rep(" ", max_len - #str)
  end

  return str
end

return M
