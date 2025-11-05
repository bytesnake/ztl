local log = require 'ztl.log'

local span = nil
local M = {}

local once = true

function M.fold_level(win_id, path) 
  --log.info("Fold level for " .. tostring(win_id) .. " " .. path .. " " .. vim.v.lnum)
  -- if either span not loaded, or current file does not have
  -- a filename, then exit (virtual files do not make sense for
  -- ztl)
  current_span = span[win_id]
  --log.info(vim.inspect(current_span))
  if current_span == nil or vim.fn.expand("%:p") == "" then
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

function M.fold_text(win_id, path)
  current_span = span[win_id]
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

function M.setup(spann, path)
  span = spann

  --vim.print(vim.inspect(span))
  --log.info(vim.inspect(span))

  local win_id = vim.api.nvim_get_current_win()

  --log.info("Setup spans for " .. tostring(win_id) .. "for file " .. path)

  --vim.opt_local.foldmethod = "syntax"
  vim.schedule(function()
	vim.opt_local.foldexpr = 'v:lua.require("ztl.folding").fold_level(' .. win_id .. ', \"' .. path .. '\")'
	vim.opt_local.foldtext = 'v:lua.require("ztl.folding").fold_text(' .. win_id .. ', \"' .. path .. '\")'
	vim.opt_local.foldmethod = "expr"
  end)
end

return M



