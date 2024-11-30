local toml = require("toml")
local fwatch = require('fwatch')

-- setup shorcuts for commonly used vim functions
local ag = vim.api.nvim_create_augroup
local au = vim.api.nvim_create_autocmd
local deep_extend = vim.tbl_deep_extend

local default_config = {
	pdf_viewer = 'okular {file}',
	url_viewer = 'firefox {url}'
}

local watcher = nil
local current_note = {}
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
		if closest < p and p <= row then
			closest = p
			index = k
		end
	end

	return current_span[index]
end

local M = {}

function M.setup(config)
  local_config = deep_extend('keep', config or {}, default_config)

  local ztl_group = ag("ztl", { clear = true })
  au({"BufEnter", "WinEnter"}, {
	  group = ztl_group,
	  pattern = { "*.md", "*.bib", "*.tex"},
	  callback = function()
		switchFile()
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
  
  	if target == nil then
  		return
  	end
  
  	succeeded, note = pcall(toml.decodeFromFile, ".ztl/cache/" .. span["target"])
  
  	local outgoing = note["outgoing"][tonumber(target["index"]) + 1]
  
  	succeeded, note = pcall(toml.decodeFromFile, ".ztl/cache/" .. outgoing["target"])
  
	if note["target"] ~= nil then
		if string.sub(note["target"], 1, 4) == "http" then
			local file = note["target"]

			local view = outgoing["view"]
			if view["anchor"] ~= nil then
				file = file .. "#" .. view["anchor"]
			end

			local cmd = config["url_viewer"](file, view["page"], view["search"])
			vim.fn.jobstart(cmd, {
				on_error = function(err)
					vim.notify("Could not start website viewer", "error")
				end
			})
		elseif note["target"]:sub(-4) == ".pdf" then
			local file = note["target"]

			local view = outgoing["view"]
			if view["anchor"] ~= nil then
				file = file .. "#" .. view["anchor"]
			end

			local cmd = config["pdf_viewer"](file, view["page"], view["search"])
			vim.fn.jobstart(cmd, {
				on_error = function(err)
					vim.notify("Could not start pdf viewer", "error")
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
		vim.api.nvim_win_set_cursor(0, {note["span"]["start"]["line"] + 1, 0})
	end
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
		end
	end


	return header
end

return M
