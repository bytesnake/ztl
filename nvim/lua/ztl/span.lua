local utils = require "ztl.utils"

local NoteSpan = {}
NoteSpan.__index = NoteSpan

function NoteSpan:new()
  local self = setmetatable({}, NoteSpan)

  -- search for .ztl config folder upwards up to current file
  wdir = vim.fn.finddir(".ztl", vim.fn.expand('%:p') .. ";/")
  if wdir == "" then
	return nil
  end

  self.wdir = vim.fn.fnamemodify(wdir, ':p')
  self.span = {}

  return self
end

function NoteSpan:cache_dir()
  return self.wdir .. "cache/"
end

function NoteSpan:update_file(fname, win_id)
  fpath = vim.fn.fnamemodify(fname, ":p")
  fpath = utils.remove_prefix(fpath, vim.fn.fnamemodify(self.wdir, ":h:h") .. "/")
  fdir = string.upper(vim.fn.sha256(fpath))
  fdir = self.wdir .. "/cache/" .. fdir

  -- decode TOML file
  self.span[win_id] = utils.toml(fdir)

  return fdir
end

function NoteSpan:note()
  local row, col = unpack(vim.api.nvim_win_get_cursor(0))
  local win_id = vim.api.nvim_get_current_win()

  if self.span[win_id] == nil then
	return 
  end

  local closest = 0
  local index = ""
  for k,v in pairs(self.span[win_id]) do
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

  return self.span[win_id][index]
end

function NoteSpan:current_target()
	local span = self:note()
	if span == nil then
	  return ""
	end

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

function NoteSpan:current_header()
	local span = self:note()
	if span == nil then
	  return ""
	end

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

return NoteSpan
