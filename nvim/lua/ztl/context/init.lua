local log = require "ztl.log"
local utils = require "ztl.utils"

---@alias path string

---@class ZtlCtx
---@field wdir path Working directory of the ZTL repository
---@field span table Span information of associated buffer
---@field source path File path relative to working directory
local ZtlCtx = {}
ZtlCtx.__index = ZtlCtx

local ctxs = {}

function ZtlCtx.current()
  local win_id = vim.api.nvim_get_current_win()
  return ctxs[win_id]
end

--- New ZTL context in reference to fpath
--- @param fpath path? Path from which to search upwards for ZTL repository
function ZtlCtx.new(fpath)
  local self = setmetatable({}, ZtlCtx)
  fpath = vim.F.if_nil(fpath, vim.fn.expand('%:p'))
  fpath = vim.fn.fnamemodify(fpath, ":p")

  -- search for .ztl config folder upwards up to current file
  local wdir = vim.fn.finddir(".ztl", fpath .. ";/")
  if wdir == "" then
	log.info("No ZTL repository found for file " .. fpath)
	return nil
  end

  -- resolve full path and remove ZTL repository folder component
  self.wdir = vim.fn.fnamemodify(wdir, ':p:h:h')

  -- find associated spanning information in ZTL repository
  fpath = self:resolve_relative(fpath)
  local fdir = string.upper(vim.fn.sha256(fpath))
  fdir = self:files_dir() .. fdir

  -- decode TOML file
  self.span = utils.toml(fdir).spans
  self.source = fdir

  -- map to current window number, for calling ZtlCtx.current
  local win_id = vim.api.nvim_get_current_win()
  ctxs[win_id] = self

  return self
end

--- Update local span file
function ZtlCtx:update()
  self.span = utils.toml(self.source).spans
end

--- Resolve file name relative to working directory
--- @param fname path File in ZTL folder
--- @return path
function ZtlCtx:resolve_relative(fname)
  fname = utils.remove_prefix(fname, self.wdir .. "/")

  return fname
end

--- Return the path to notes folder
--- @return path
function ZtlCtx:notes_dir()
  return self.wdir .. "/.ztl/notes/"
end

--- Return path to files folder
--- @return path
function ZtlCtx:files_dir()
  return self.wdir .. "/.ztl/files/"
end

--- Open note
--- @param key Key Key of note
--- @return table?
function ZtlCtx:get_note(key)
  return utils.toml(self:notes_dir() .. key)
end

--- Current note at cursor position and focused window
--- @return table? Span information of current note
function ZtlCtx:note()
  local row, _ = unpack(vim.api.nvim_win_get_cursor(0))

  if self.span == nil then
	return nil
  end

  local closest = 0
  local index = ""
  for k,_ in pairs(self.span) do
	local parts = {k:match'(%d+)%:(%d+)'}
	local p = tonumber(parts[1])
	if p == nil then
	  return nil
	end

	if closest < p and p <= row then
	  closest = p
	  index = k
	end
  end

  return self.span[index]
end

--- Return current target
--- @return string Span information of current target
function ZtlCtx:current_target()
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

--- Return current header
--- @return string 
function ZtlCtx:current_header()
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

return { ZtlCtx = ZtlCtx }
