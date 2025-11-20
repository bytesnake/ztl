local utils = require "ztl.utils"
local log = require("ztl.log")
local ZtlCtx = require("ztl.context").ZtlCtx

local M = {}

-- Forward follow links at current buffer and cursor position
local function note_follow()
  local ctx = ZtlCtx.current()
  local note_span = ctx:note()
  local note = utils.toml(ctx:notes_dir() .. note_span["target"])
  if note == nil then
	return
  end

  local row, col = unpack(vim.api.nvim_win_get_cursor(0))
  -- parse possible links and check whether we are in range of one
  local target
  for k,v in pairs(note_span["outgoing"]) do
  	local parts = {k:match'(%d+)%:(%d+)%,(%d+)%:(%d+)'}
  	if #parts == 4 and tonumber(parts[1]) == row and
  		tonumber(parts[2]) < col + 2 and tonumber(parts[4]) > col then
  		target = v
  	end
  end

  local view = {}
  -- if we are not in the range of any link, open selector
  if target == nil then
	require("ztl.fncs").find_notes(note.id, { mode = "forward", action = "normal", ctx = ctx })
  else
	local outgoing = note["outgoing"][tonumber(target["index"]) + 1]
	if vim.tbl_count(outgoing.view) == 0 then
		utils.open(ctx, "key:" .. outgoing.target, nil)
	else
		local note = utils.toml(ctx:notes_dir() .. outgoing.target)
		utils.open(ctx, note.resource, outgoing.view)
	end
  end
end

local function note_retreat()
  local ctx = ZtlCtx.current()
  local note_span = ctx:note()

  require("ztl.fncs").find_notes(note_span.target, { mode = "backward", action = "normal", ctx = ctx })
end

local function note_history()
  local ctx = ZtlCtx.current()
  local note_span = ctx:note()

  require("ztl.fncs").find_notes(note_span.target, { mode = "history", action = "normal", ctx = ctx })
end

local function open_resource()
  local ctx = ZtlCtx.current()
  local note_span = ctx:note()
  local note = utils.toml(ctx:notes_dir() .. note_span.target)
  if note == nil then return end

  local row, col = unpack(vim.api.nvim_win_get_cursor(0))
  -- parse possible links and check whether we are in range of one
  local target
  for k,v in pairs(note_span["outgoing"]) do
  	local parts = {k:match'(%d+)%:(%d+)%,(%d+)%:(%d+)'}
  	if #parts == 4 and tonumber(parts[1]) == row and
  		tonumber(parts[2]) < col + 2 and tonumber(parts[4]) > col then
  		target = v
  	end
  end

  if target == nil then
	  if note.resource ~= nil then
		require("ztl.log").info(note.resource)
		  require("ztl.utils").open(ctx, note.resource, nil)
	  end
  else
	local outgoing = note["outgoing"][tonumber(target["index"]) + 1]
	note = utils.toml(ctx:notes_dir() .. outgoing.target)
	utils.open(ctx, note.resource, outgoing.view)
  end
end

-- Setup mappings to `<Plug>Ztl` namespace
function M.setup()
  vim.keymap.set("n", "<Plug>ZtlFollow", note_follow)
  vim.keymap.set("n", "<Plug>ZtlRetreat", note_retreat)
  vim.keymap.set("n", "<Plug>ZtlHistory", note_history)
  vim.keymap.set("n", "<Plug>ZtlOpenResource", open_resource)

  -- insert new note ID (random 6 digits character)
  vim.keymap.set('i', '<Plug>ZtlInsertKey', function()
      local uuid = utils.string_random(6)

      local row, col = unpack(vim.api.nvim_win_get_cursor(0))
      vim.api.nvim_buf_set_text(0, row - 1, col, row - 1, col, { uuid .. " " })
      vim.api.nvim_win_set_cursor(0, { row, col + 7 })
  end)

end

return M
