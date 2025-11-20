local utils = require "ztl.utils"
local log = require("ztl.log")
local ZtlCtx = require("ztl.context").ZtlCtx
local pickers = require("telescope.pickers")
local Stack = require "ztl.telescope"

local M = {}

--- Go to position
---@alias Position "header" | "beginning" | "end" 
---@alias GoToMode "insert" | "normal" | "keep" In what mode should we be

--- Insert a new note either in given file or relative to a note
---
---@param where 
---| 	{ key: Key, relative?: "beginning" | "end" } | 
---| 	{ file: path, relative?: "first-child" | "last-child" | "before" | "after" }
---@param title string
---
---@usage >lua
--- 	-- insert note at the beginning of file "Zettel.md"
--- 	require("ztl.fncs").insert_note({ file = "Zettel.md", relative = "beginning" })
--- 	-- OR
--- 	-- insert note as first child of note gdf789x
--- 	require("ztl.fncs").insert_note({ key = "gdf789x", relative = "beginning" })
---
--- <
function M.insert_note(where, title)
  -- only works for markdown for now
  if vim.bo.filetype ~= "markdown" then
  	return
  end

  if where.key ~= nil then
	local note = utils.toml(ZtlCtx.current():notes_dir() .. where.key)
  	vim.fn.cursor(tonumber(note["span"]["start"]["line"]), 1)
  	local line = vim.fn.search("^#", "nw")
  	local content = vim.api.nvim_buf_get_lines(0, line - 1, line, false)[1]
  	local leading_hash = content:match("^#+")

  	vim.api.nvim_buf_set_lines(0, line-1, line-1, false, {leading_hash .. " " .. utils.string_random(6) .. " " .. title, "", "", ""})
  	vim.fn.cursor(line + 2, 1)
  	vim.schedule(function() vim.cmd("normal! zO") end)
  	return
  end

  vim.api.nvim_buf_set_lines(0, -1, -1, false, {"# " .. utils.string_random(6) .. " " .. title, "", ""})
  vim.cmd("normal G")
end

--- Goto note with key in a given window 
---@param winnr integer Window number (winnr = 0 indicates current window)
---@param key Key Note key 
---@param opts? {position: GoToPosition, mode: GoToMode, ctx: ZtlCtx?, open_folds: boolean}
function M.goto(winnr, key, opts)
	opts = vim.tbl_deep_extend("force", {
		position = "header", mode = "keep", ctx = nil, open_folds = true }, opts or {})

	local ctx = vim.F.if_nil(opts.ctx, ZtlCtx.current())
	if ctx == nil then
		return
	end

	local note = ctx:get_note(key)
	local span = note.span

	local function jump_to()
	  if opts.position == "header" then
		  vim.api.nvim_win_set_cursor(0, {span.start.line, 0})
	  elseif opts.position == "beginning" then
		  vim.api.nvim_win_set_cursor(0, {span.start.line+1, 0})
	  elseif opts.position == "end" then
		  vim.api.nvim_win_set_cursor(0, {span["end"].line+1, 0})
	  end

	  -- press all thumbs that this happens after loading folding 
	  if opts.open_folds then
		  vim.schedule(function() vim.cmd("normal! zO") end)
	  end

	  -- check that we are in the desired mode
	  local mode = vim.api.nvim_get_mode().mode
	  if mode == "n" and opts.mode == "insert" then
		vim.cmd("startinsert!")
	  elseif mode == "i" and opts.mode == "normal" then
		vim.cmd("stopinsert!")
	  end
	end
	local buf_name = vim.api.nvim_buf_get_name(winnr)
	if string.find(buf_name, span.source, 1, true) then
		jump_to()
	else
		vim.api.nvim_create_autocmd('BufWinEnter', {
			pattern = span.source,
			callback = jump_to,
			once = true
		})

		vim.cmd("edit " .. ctx.wdir .. "/" .. span.source)
	end
end

---@alias Source {key: Key}|{keys: Key[]}|nil
---@alias Mode
---| "forward" # Display outgoing _or_ children notes
---| "backward" # Display incoming _or_ parent note
---| "resource" # Attached resources of the current note
---| "history" # Past changes to the note or set of notes
---@alias Action
---| "insert" # create a new link at current position
---| "visual" # replace visual selection with selected note and use selection as link label
---| "normal" # open selected note in current buffer

--- Telescoping all notes, a list of notes or a single note
--- @param source Source Source of notes
--- @param opts {mode: Mode, action: Action, ctx?: ZtlCtx}? Optional modifiers
function M.find_notes(source, opts)
  -- default optional parameters
  opts = vim.tbl_deep_extend("force",
	  { mode = "forward", action = "normal"}, opts or {})

  -- either use context of parameters or get associated with current window
  opts.ctx = vim.F.if_nil(opts.ctx, ZtlCtx.current())
  local state = Stack:new(source, opts)

  -- save selection range for visual action
  if opts.action == "visual" then
	opts.range = {vim.fn.getpos("v"), vim.fn.getpos(".") }
  end

  local tel_opts = {
	  prompt_title = prompt_title,
	  layout_strategy = "vertical",
	  prompt_position = "top",
	  layout_config = {
		  height = {padding = 0},
		  width = 0.75,
		  preview_height = 0.4,
		  preview_cutoff = 1,
		  mirror = true,
	  }
  }

  opts.telescope = tel_opts

  pickers
  .new(tel_opts, {
	finder = state.stack[1].finder,
	sorter = require("ztl.telescope.utils").post_sorter(opts),
	attach_mappings = require("ztl.telescope.actions")(state, opts),
	previewer = state.stack[1].previewer,
  }):find()
end

--- Display current scheduled notes in Telescope picker
function M.schedule()
	local ctx = ZtlCtx.current()
	require("ztl.telescope.schedule").schedule(ctx)
end

return M
