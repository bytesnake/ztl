local entry_display = require('telescope.pickers.entry_display')
local previewers = require('telescope.previewers')
local finders = require("telescope.finders")
local utils = require "ztl.utils"
local log = require "ztl.log"

local M = {}

local function const_data(arr, displayer)
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

local function notes_all(opts)
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

local function insert_key(arr, opts, target_key, dir, view)
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

local function notes_single(key, opts)
  local note = utils.toml(opts.ctx:notes_dir() .. key)
  if note == nil then
    error("Note " .. key .. "not found")
	return
  end

  local arr = {}
  if opts.mode == "forward" then
	for _,v in pairs(note["outgoing"]) do
		insert_key(arr, opts, v["target"], "outgoing", v["view"])
	end

	for _,v in pairs(note["children"]) do
		insert_key(arr, opts, v, "children")
	end
  elseif opts.mode == "backward" then
	for _,v in pairs(note["incoming"]) do
		insert_key(arr, opts, v, "incoming")
	end

	if note["parent"] ~= nil then
		insert_key(arr, opts, note["parent"], "parent")
	end
  end

  local function make_display(ent)

	local displayer
	if ent.value.view ~= nil and next(ent.value.view) ~= nil then
	  displayer = entry_display.create {
		separator = "",
		items = {
		  { width = 6 }, -- section
		  { width = 4 }, -- section
		  { width = 30 }, -- section
		  { remaining = true }, -- header
		},
	  }
	else
	  displayer = entry_display.create {
		separator = "",
		items = {
		  { width = 6 }, -- section
		  { width = 4 }, -- section
		  --{ width = 30 }, -- section
		  { remaining = true }, -- header
		},
	  }
	end

	local picto
	if ent.value.dir == "children" then
	  picto = ""
	elseif ent.value.dir == "parent" then
	  picto = ""
	elseif ent.value.dir == "outgoing" then
	  picto = "" 
	elseif ent.value.dir == "incoming" then
	  picto = ""
	end

	if ent.value.kind == nil then
	  ent.value.kind = "note"
	end

	local display = {
	  -- text, highlight group
	  { ent.value.kind, "notePreviewKind" },
	  { " " .. picto .. "  ", "notePreviewPicto" },
	  { ent.value.header, "notePreviewHeader" },
	}
	if ent.value.view ~= nil and next(ent.value.view) ~= nil then
	  table.insert(display, {
		string.format("%20s", require("ztl.telescope.utils").view_to_string(ent.value.view)), "markdownUrl"})
	end

	return displayer(display)
  end

  return arr, make_display
end

local function notes_many(keys, opts)
  local arr = {}
  for _,v in pairs(keys) do
	insert_key(arr, opts, v, nil)
  end

  local function make_display(entry)
	local displayer = entry_display.create {
	  separator = " ▏",
	  items = {
		{ width = 8 }, -- section
		{ remaining = true }, -- header
	  },
	}

	if entry.value.kind == nil then
	  entry.value.kind = "note"
	end

	return displayer {
	  -- text, highlight group
	  { entry.value.kind, "notePreviewKind" },
	  { entry.value.header, "notePreviewHeader" },
	}
  end

  return arr, make_display
end

function M.finder(source, opts)
  if source == nil then
	return const_data(notes_all(opts))
  elseif type(source) == "string" then
	return const_data(notes_single(source, opts))
  elseif type(source) == "table" then
	return const_data(notes_many(source, opts))
  end
end

function M.previewer(opts)
  return previewers.vim_buffer_vimgrep:new(opts)
end

return M
