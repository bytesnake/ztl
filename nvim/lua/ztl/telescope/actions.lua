local actions_state = require('telescope.actions.state')
local utils = require("ztl.utils")
local log = require "ztl.log"

local function action_open(bfnr, ctx, curr_note)
  local picker = actions_state.get_current_picker(bfnr)
  local selection = actions_state.get_selected_entry()

  -- insert new note as first child, if nothing was selected
  if selection == nil then
	local prompt = picker:_get_prompt()
	picker:close_windows()

	require("ztl.fncs").insert_note({
	  key = curr_note.key, relative = "first-child" }, prompt)

	return
  end

  local note
  if selection.ordinal.note == nil then
	note = utils.toml(ctx:notes_dir() .. selection.ordinal.key)
  else
	note = selection.ordinal.note
  end

  if note == nil then
    return
  end

  if note.resource ~= nil then
    utils.open(ctx, note.resource, selection.ordinal.view or {})
  else
	picker:close_windows()
    utils.open(ctx, "key:" .. note.id, {})
  end
end

local function action_insert(bfnr)
  local selection = actions_state.get_selected_entry()
  local picker = actions_state.get_current_picker(bfnr)

  picker:close_windows()

  vim.schedule(function()
	  local extension = vim.fn.expand("%:e")

	  local key
	  if extension == "md" then
	    key = "[]("..selection.ordinal.key .. utils.format_view(selection.ordinal.view) .. ")"
	  elseif extension == "tex" then
	    key = "\\r{"..selection.ordinal.key .. utils.format_view(selection.ordinal.view) .. "}{}"
	  else
	    return
	  end

	  local row, col = unpack(vim.api.nvim_win_get_cursor(0))
	  log.info(row, col)
	  vim.api.nvim_buf_set_text(0, row - 1, col + 1, row - 1, col + 1, { key .. " " })

	  if extension == "md" then
	    vim.api.nvim_win_set_cursor(0, { row, col + 2 })
	  elseif extension == "tex" then
	    vim.api.nvim_win_set_cursor(0, { row, col + #key })
	  end

	  vim.cmd("startinsert")
  end)
end

local function action_replace(bfnr, opts)
  local selection = actions_state.get_selected_entry()
  local picker = actions_state.get_current_picker(bfnr)
  local range = opts.range

  picker:close_windows()

  vim.schedule(function()
	  local key = selection.ordinal.key
	  -- Extract the selected text
	  local lines = vim.api.nvim_buf_get_text(0, range[1][2] - 1, range[1][3] - 1, range[2][2] - 1, range[2][3], {})

	  local selected_text = table.concat(lines, "\n")

	  local extension = vim.fn.expand("%:e")

	  local surrounded_text
	  if extension == "md" then
		-- Surround the selected text with [](key)
		surrounded_text = string.format("[%s](%s)", selected_text, key)
	  elseif extension == "tex" then
		surrounded_text = string.format("\\r{%s}{%s}", key, selected_text)
	  else
		return
	  end

	  -- Replace the selected text with the surrounded text
	  vim.api.nvim_buf_set_text(0, range[1][2] - 1, range[1][3] - 1, range[2][2] - 1, range[2][3], vim.split(surrounded_text, "\n"))
  end)
end

-- Follow current selected note
function M.follow(bfnr, state, opts)
  local current_picker = actions_state.get_current_picker(bfnr)

  -- save current prompt for reuse, when we return
  state:current().last_prompt = current_picker:_get_prompt()
  state:current().last_pos = current_picker:get_selection_row()

  local selection = actions_state.get_selected_entry()

  -- if selection is null or this is a view of a resource, we can't follow
  if selection == nil or selection.ordinal.target_str ~= nil then
	return
  end

  --
  -- check if following note has same key, otherwise crop stack
  if state.idx < #state.stack and (state.stack[state.idx + 1].current == nil or state.stack[state.idx + 1].current.id ~= selection.ordinal.key) then
	for i = #state.stack, state.idx + 1, -1 do
	  table.remove(state.stack, i)
	end
  end

  --log.info(selection.ordinal.key)
  -- if next index exceeds number of finders, append
  -- one generated from selected entry
  state.idx = state.idx + 1
  if state.idx > #state.stack then
	-- if ordinal is a string, the action originated from Git follow
	if type(selection.ordinal) == "string" then
	  state:push(require("ztl.telescope.history").list_from_commit(selection.value), { mode = "forward", ctx = opts.ctx})
	  current_picker.prompt_border:change_title("Notes of commmit " .. selection.value)
	else
	  state:push(selection.ordinal.key, { mode = "forward", ctx = opts.ctx })
	  current_picker.prompt_border:change_title(state:current().current.header)
	end
  end

  -- update new finder
  current_picker:refresh(state:current().finder)
  current_picker:set_prompt(state:current().last_prompt)
  current_picker.previewer = state:current().previewer
  current_picker:refresh_previewer()
end

-- Retreat to previously selected note
function M.retreat(bfnr, state)
  if state.idx == 1 then
	return
  end

  local current_picker = actions_state.get_current_picker(bfnr)
  state:pop()

  -- update new finder
  if state:current().current ~= nil then
	current_picker.prompt_border:change_title(state:current().current.header)
  else
	current_picker.prompt_border:change_title("All Notes")
  end

  current_picker:refresh(state:current().finder)
  current_picker:set_prompt(state:current().last_prompt)
  vim.defer_fn(function()
	current_picker:set_selection(state:current().last_pos)
	current_picker.previewer = state:current().previewer
	current_picker:refresh_previewer()
  end, 60)
  --require("ztl.log").info(vim.inspect(state.stack))
end

--- Change the mode of last position
---
---@param mode Mode New Mode
function M.change_mode(bfnr, state, mode, opts)
  -- if we are showing all notes, there is no distinction
  -- between incoming and outgoing links
  if state:current().current == nil then
	return
  end

  local finder, previewer = state:create_new(state:current().current.id, { mode = mode, ctx = opts.ctx })

  -- replace last finder with new outgoing links
  state:current().finder = finder
  state:current().previewer = previewer

  local current_picker = actions_state.get_current_picker(bfnr)
  current_picker:refresh(state:current().finder)
  vim.defer_fn(function()
	current_picker.previewer = state:current().previewer
	current_picker:refresh_previewer()
  end, 20)
end

return function(state, opts)
  return function(prompt_bufnr, map)
	if opts.action == "normal" then
	  map('i', '<CR>', function() action_open(prompt_bufnr, opts.ctx, state) end)
	elseif opts.action == "insert" then
	  map('i', '<CR>', function() action_insert(prompt_bufnr) end)
	elseif opts.action == "visual" then
	  map('i', '<CR>', function() action_replace(prompt_bufnr, opts) end)
	end

	map('i', "<Right>", function() M.follow(prompt_bufnr, state, opts) end)
	map('i', "<Left>", function() M.retreat(prompt_bufnr, state) end)

	map('i', "<C-f>", function() M.change_mode(prompt_bufnr, state, "forward", opts) end)
	map('i', "<C-b>", function() M.change_mode(prompt_bufnr, state, "backward", opts) end)
	map('i', "<C-j>", function() M.change_mode(prompt_bufnr, state, "history", opts) end)

	return true
  end
end
