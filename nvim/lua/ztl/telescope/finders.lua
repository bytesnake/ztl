local entry_display = require('telescope.pickers.entry_display')
local previewers = require('telescope.previewers')
local finders = require("telescope.finders")
local utils = require "ztl.utils"

local M = {}

function M.const_data(arr, displayer)
  return finders.new_table {
	results = arr,
	entry_maker = function(entry)
	  local res = vim.split(entry["target"] or ":", ":")

	  return {
		value = entry,
		display = displayer,
		ordinal = entry,
		filename = res[1],
		lnum = tonumber(res[2]),
	  }
	end
  }
end

function M.notes_all(opts)
  -- parse global list of notes from ztl binary
  local result = vim.system({"ztl", "--root", opts.ctx.wdir, "--format", "json", "list"}, { text = true }):wait()
  local arr = vim.json.decode(result.stdout)
  if arr["Err"] ~= nil then
	  for k in pairs(arr["Err"]) do
		  vim.notify(table.concat(arr["Err"][k], "\n"), "error")
	  end

	  return
  end

  local function make_display(entry)
	local displayer = entry_display.create {
	  separator = " ▏",
	  items = {
		{ width = 8 }, -- section
		{ remaining = true }, -- header
	  },
	}

	return displayer {
	  -- text, highlight group
	  { entry.value.kind, "notePreviewKind" },
	  { entry.value.header, "notePreviewHeader" },
	}
  end

  return arr.Ok.List.notes, make_display
end

function M.notes_single(key, opts)
  local arr = {}
  local function insert_key(target_key, dir, view)
	  local target = utils.toml(opts.ctx:notes_dir() .. target_key)
	  if target == nil then
		return
	  end

	  table.insert(arr, {
		key = target["id"],
		kind = target["kind"],
		header = target["header"],
		target = target["span"]["source"] .. ":" .. target["span"]["start"]["line"],
		note = target,
		dir = dir,
		view = view,
	  })
  end

  local note = utils.toml(opts.ctx:notes_dir() .. key)
  if note == nil then
    error("Note " .. key .. "not found")
	return
  end

  if opts.mode == "forward" then
	for _,v in pairs(note["outgoing"]) do
		insert_key(v["target"], "outgoing", v["view"])
	end

	for _,v in pairs(note["children"]) do
		insert_key(v, "children")
	end
  elseif opts.mode == "backward" then
	for _,v in pairs(note["incoming"]) do
		insert_key(v, "incoming")
	end

	if note["parent"] ~= nil then
		insert_key(note["parent"], "parent")
	end
  end

  return arr
end

function M.finder(source, opts)
  if source == nil then
	return const_data(notes_all(opts))
  elseif type(source) == "string" then
	return const_data(notes_single(source, opts))
  elseif type(source) == "table" then
	return const_data(notes_single(source, opts))
  end
end

function M.previewer(opts)
  return previewers.vim_buffer_vimgrep:new(opts)
end

return M
