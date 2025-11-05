local utils = require "ztl.utils"
local log = require("ztl.log")
local M = {}

function M.forward_follow(span)
  local note_span = span:note()
  local note = utils.toml(span:cache_dir() .. note_span["target"])
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
  if note.resource == nil and target == nil then
	require("ztl.telescope").find_notes(span, note_span, false)
	return
  elseif note.resource == nil then
  	local outgoing = note["outgoing"][tonumber(target["index"]) + 1]
  	view = outgoing["view"]
	utils.open(span, "key:" .. outgoing.target, view)
  else
    utils.open(span, note.resource, view)
  end
end

function M.backward_follow(span)
  local note_span = span:note()
  --local note = utils.toml(span:cache_dir() .. note["target"])

  require("ztl.telescope").find_notes(span, note_span, true)
end

return M
