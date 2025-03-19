local toml = require("toml")
local fwatch = require('fwatch')

-- setup shorcuts for commonly used vim functions
local ag = vim.api.nvim_create_augroup
local au = vim.api.nvim_create_autocmd
local deep_extend = vim.tbl_deep_extend

local default_config = {
	pdf_viewer = 'okular {file}',
	url_viewer = 'firefox {url}',
	suggestion = {
		prompts = {
			"Can you suggest possible improvements?",
			"What other possible atomic notes could be connected to this one?",
			"Can you come up with other topic to write about?",
			"Can you write a table of content with three levels for a book based on the note but extend it also to possible other topics?",
			"Custom Prompt",
		},
		callback = function(prompt, cb)
			local cmd = "curl \"https://generativelanguage.googleapis.com/v1beta/models/gemini-1.5-flash:generateContent?key=\" -H 'Content-Type: application/json' -X POST -d '{\"contents\": [{\"parts\":[{\"text\": \"" .. prompt .. "\"}] }]}'"

			vim.fn.jobstart(cmd, {
				on_error = function(err)
					vim.notify("Could not start curl", "error")
				end,
				stdout_buffered = true,
				on_stdout = function(_, stdout)
					local tmp = table.concat(stdout, "\n")
					local buf = vim.json.decode(tmp)

					local out = ""
					for _,text in ipairs(buf["candidates"][1]["content"]["parts"])do
						out = out .. text["text"] .. "\n\n"
					end

					cb(out)
				end,
			})
		end
	}
}

local watcher = nil
local cur_note = {}
local current_fdl = ""

function switchFile()
	if watcher ~= nil then
		fwatch.unwatch(watcher)
	end

	-- search for .ztl config folder upwards up to current file
	current_fdl = vim.fn.finddir(".ztl", vim.fn.expand('%:p') .. ";/")
	current_fdl = vim.fn.fnamemodify(current_fdl, ':p')
	if current_fdl == "" then
		current_span = {}
		return
	end

	local fname = vim.fn.expand('%')
	local fdir = string.upper(vim.fn.sha256(fname))
	local fdir = current_fdl .. "/cache/" .. fdir

	function update()
		local succeeded, notes = pcall(toml.decode, tomlStr)

		-- Decode from file
		succeeded, spans = pcall(toml.decodeFromFile, fdir)
		current_span = spans

		vim.opt.foldexpr = 'v:lua.get_my_foldlevel()'
		vim.opt.foldtext = 'v:lua.get_fold_text()'

		return fdir
	end

	update()

	local defer = nil
	watcher = fwatch.watch(fdir, {on_event = function()
	  if not defer then -- only set once in window
		defer = vim.defer_fn(function()
		  defer = nil -- clear for next event out side of window
		  update() -- do work
		end, 100)  -- run in 100 ms, probably this can be much lower.
	  end
	end})
end

_G.get_my_foldlevel = function()
	if current_span == nil then
		return 0
	end

	local num_fold = 0
	for k,v in pairs(current_span) do
  		local parts = {k:match'(%d+)%:(%d+)'}
		if vim.v.lnum >= tonumber(parts[1]) and vim.v.lnum <= tonumber(parts[2]) then
			num_fold = num_fold + 1
		end
	end

	for k,v in pairs(current_span) do
  		local parts = {k:match'(%d+)%:(%d+)'}
		if vim.v.lnum == tonumber(parts[1]) then
			return ">" .. num_fold
		end
		if vim.v.lnum == tonumber(parts[2]) then
			return "<" .. num_fold
		end
	end

	return num_fold
end

_G.get_fold_text = function()
	if current_span == nil then
		return ""
	end

	local longest_target = 0
	for k,v in pairs(current_span) do
		local target = v["target"]
		if v["kind"] ~= nil then
			target = v["kind"] .. "(" .. target .. ")"
		end
		
		if string.len(target) > longest_target then
			longest_target = string.len(target)
		end
	end

	for k,v in pairs(current_span) do
  		local parts = {k:match'(%d+)%:(%d+)'}
		if vim.v.foldstart == tonumber(parts[1]) and vim.v.foldend == tonumber(parts[2]) then
			local target = v["target"]
			if v["kind"] ~= nil then
				target = v["kind"] .. "(" .. target .. ")"
			end
			local num_whitespace = longest_target - string.len(target) + 2
				
			return "╠" .. string.rep("═", vim.v.foldlevel) .. " " .. target .. string.rep(" ", num_whitespace) .. v["header"]
		end
	end

	return "Unknown"
end

function current_note()
	local row, col = unpack(vim.api.nvim_win_get_cursor(0))

	local closest = 0
	local index = ""
	for k,v in pairs(current_span) do
  		local parts = {k:match'(%d+)%:(%d+)'}
		local p = tonumber(parts[1])
		if p == nil then
			return
		end
			
		if closest < p and p <= row then
			closest = p
			index = k
		end
	end

	return current_span[index]
end

local buffer_number = -1

local function log(_, data)
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

local function open_buffer()
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

local sets = {{97, 122}, {48, 57}} -- a-z, 0-9
local function string_random(chars)
	local str = ""
	for i = 1, chars do
		math.randomseed(os.clock() ^ 5)
		local charset = sets[ math.random(1, #sets) ]
		str = str .. string.char(math.random(charset[1], charset[2]))
	end
	return str
end

local M = {}

function M.setup(config)
  local_config = deep_extend('keep', config or {}, default_config)

  vim.cmd([[
  	syn region markdownLink matchgroup=markdownLinkDelimiter start="(" end=")" contains=markdownUrl keepend contained conceal
  	syn region markdownLinkText matchgroup=markdownLinkTextDelimiter start="!\=\[\%(\%(\_[^][]\|\[\_[^][]*\]\)*]\%( \=[[(]\)\)\@=" end="\]\%( \=[[(]\)\@=" nextgroup=markdownLink,markdownId skipwhite contains=@markdownInline,markdownLineStart concealends
  	set conceallevel=2
  	highlight markdownLinkText ctermfg=red guifg=#076678 cterm=bold term=bold gui=bold
  	set concealcursor=nc
  ]])

  local ztl_group = ag("ztl", { clear = true })
  au({"BufEnter", "WinEnter"}, {
	  group = ztl_group,
	  pattern = { "*.md", "*.bib", "*.tex"},
	  callback = function()
		switchFile()
	  end
  })
  au({"CursorMoved"}, {
	  group = ztl_group,
	  pattern = { "*.md", "*.bib", "*.tex"},
	  callback = function()
		local note = current_note()
		if note == nil then
			return
		end

		if cur_note["target"] ~= note["target"] then
			local file = io.open(".ztl/cache/" .. note["target"] .. ".sixel.show", "w")
			file.close()
		end
		cur_note = note
	  end
  })

  -- check for .ztl subfolder and modify lualine if exists
  folder = vim.fn.finddir(".ztl", vim.fn.expand('%:p') .. ";/")
  if folder == "" then
	  return;
  end

  require('lualine').setup({
  	sections = { 
  		lualine_b = {current_target},
  		lualine_c = {current_header},
  		lualine_x = {},
  		lualine_y = {},
  	},
  })
  
  vim.keymap.set('n', 'gb', function() 
  	local span = current_note()
  	succeeded, note = pcall(toml.decodeFromFile, ".ztl/cache/" .. span["target"])
  
	local items = {}
	for _,k in ipairs(note["incoming"]) do
		succeeded, target = pcall(toml.decodeFromFile, ".ztl/cache/" .. k)
		table.insert(items, {filename=target["span"]["source"], lnum=target["span"]["start"]["line"], end_lnum=target["span"]["end"]["line"], text=target["header"]})
	end
	vim.fn.setloclist(0, items)
  end)

  vim.keymap.set('n', 'gs', function()
	  vim.ui.select(local_config["suggestion"]["prompts"], {
			prompt = 'Which prompt to use',
		}, function(choice)
			local span = current_note()
			succeeded, note = pcall(toml.decodeFromFile, ".ztl/cache/" .. span["target"])

			local html = note["html"]:gsub("([%c%z\"'\\{}])", "\\%1")

			local cb = function(elm) 
				local prompt = elm .. "\n\nThe following contains the note in HTML5 formatted with possible MathML\nPlease reply in markdown and maximum linewidth of 70 characters\n" .. html

				local resp = local_config["suggestion"]["callback"](prompt, function(resp)
					open_buffer()
					log("", resp)
				end)
			end

			if choice == "Custom Prompt" then
				vim.ui.input({ prompt = "Enter custom prompt: " }, function(input)
					cb(tostring(input))
				end)
			else
				cb(choice)
			end
		end)
  end)

  vim.keymap.set('n', 'gf', function() 
  	local span = current_note()
  	local row, col = unpack(vim.api.nvim_win_get_cursor(0))
  
  	local target
  	for k,v in pairs(span["outgoing"]) do
  		local parts = {k:match'(%d+)%:(%d+)%,(%d+)%:(%d+)'}
  		if #parts == 4 and tonumber(parts[1]) == row and
  			tonumber(parts[2]) < col + 2 and tonumber(parts[4]) > col then
  
  			target = v
  		end
  	end
  
  	succeeded, note = pcall(toml.decodeFromFile, ".ztl/cache/" .. span["target"])
  
	local view = {}
	if note["target"] == nil and target == nil then
		local items = {}
		for k,v in pairs(span["outgoing"]) do
			local outgoing = span["outgoing"][k]
			succeeded, target = pcall(toml.decodeFromFile, ".ztl/cache/" .. outgoing["target"])
			local index = outgoing["index"]
			local label = note["outgoing"][index+1]
			table.insert(items, {filename=v["source"], lnum=target["span"]["start"]["line"], end_lnum=target["span"]["end"]["line"], text=label["label"] .. " -> " .. v["header"]})
		end
		vim.fn.setloclist(0, items)
  		return
	elseif note["target"] == nil then
		local outgoing = note["outgoing"][tonumber(target["index"]) + 1]
		view = outgoing["view"]
		succeeded, note = pcall(toml.decodeFromFile, ".ztl/cache/" .. outgoing["target"])
	end
  
	if note["target"] ~= nil then
		if note["target"]:sub(-4) == ".pdf" then
			local file = note["target"]

			if view["anchor"] ~= nil then
				file = file .. "#" .. view["anchor"]
			end

			local cmd = config["pdf_viewer"](file, view["page"], view["search"])
			vim.fn.jobstart(cmd, {
				on_error = function(err)
					vim.notify("Could not start pdf viewer", "error")
				end
			})
		elseif string.sub(note["target"], 1, 4) == "http" then

			local file = note["target"]

			if view["anchor"] ~= nil then
				file = file .. "#" .. view["anchor"]
			end

			local cmd = config["url_viewer"](file, view["page"], view["search"])
			vim.fn.jobstart(cmd, {
				on_error = function(err)
					vim.notify("Could not start website viewer", "error")
				end
			})
		elseif note["target"]:sub(-3) == ".md" then
			local file = vim.fs.normalize(note["target"])
			if file[0] ~= "/" then
				file = vim.fs.normalize(vim.fn.fnamemodify(current_fdl, ":h:h") .. "/" .. file)
			end

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
		else
			vim.notify("Target " .. note["target"] .. " not supported", "error")
		end
	else
		vim.cmd("normal! m'")
		vim.cmd("edit " .. note["span"]["source"])
		vim.api.nvim_win_set_cursor(0, {note["span"]["start"]["line"], 0})
	end
  end)

  vim.keymap.set('i', '<C-z>', function() 
	  local uuid = string_random(6)
	  
	  local row, col = unpack(vim.api.nvim_win_get_cursor(0))
	  vim.api.nvim_buf_set_text(0, row - 1, col, row - 1, col, { uuid .. " " })
	  vim.api.nvim_win_set_cursor(0, { row, col + 7 })
  end)
end

function current_target()
	local span = current_note()
	local row, col = unpack(vim.api.nvim_win_get_cursor(0))

	local target = span["target"]
	for k,v in pairs(span["outgoing"]) do
		local parts = {k:match'(%d+)%:(%d+)%,(%d+)%:(%d+)'}
		if #parts == 4 and tonumber(parts[1]) == row and
			tonumber(parts[2]) < col + 2 and tonumber(parts[4]) > col then

			if v["source"]:sub(-3) == ".md" then
				target = "%#DiffAdd# " .. v["target"]
			else
				target = "%#MiniHipatternsNote# " .. v["target"]
			end
		end
	end

	return target
end

function current_header()
	local span = current_note()
	local row, col = unpack(vim.api.nvim_win_get_cursor(0))

	local header = span["header"]
	for k,v in pairs(span["outgoing"]) do
		local parts = {k:match'(%d+)%:(%d+)%,(%d+)%:(%d+)'}
		if #parts == 4 and tonumber(parts[1]) == row and
			tonumber(parts[2]) < col + 2 and tonumber(parts[4]) > col then

			local fname = vim.fn.expand('%')
			if fname ~= v["source"] then
				header = v["source"] .. " ~ " .. v["header"]
			else
				header = v["header"]
			end

			if v["view"] ~= nil then
				header = header .. " ~ " .. v["view"]
			end
		end
	end


	return header
end

return M
