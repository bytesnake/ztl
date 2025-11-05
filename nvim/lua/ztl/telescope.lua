local pickers = require("telescope.pickers")
local finders = require("telescope.finders")
local sorters = require("telescope.sorters")
local previewers = require("telescope.previewers")
local entry_display = require('telescope.pickers.entry_display')
local actions_state = require('telescope.actions.state')
local actions = require('telescope.actions')
local putils = require "telescope.previewers.utils"
local make_entry = require "telescope.make_entry"
local conf = require("telescope.config").values
local utils = require("ztl.utils")
local log = require "ztl.log"
local Path = require "plenary.path"

local M = {}

local function post_sorter(opts)
  opts = opts or {}
  -- We can use `fzy_sorter` for the actual fuzzy matching.
  local fzy_sorter = sorters.get_fzy_sorter(opts)

  return sorters.Sorter:new({
    -- Allow us to filter entries as well as sorting them.
    discard = true,

    scoring_function = function(_, prompt, entry)
      -- This mimics a standard fuzzy sorting on the entry title.
      return fzy_sorter:scoring_function(prompt, entry.header)
    end,

    -- We could also specify a highlighter. The highlighter works fine in this case,
    -- but if we modify `scoring_function` we have to modify this too.
    -- I admit, I currently don't use a highlighter for my posts finder.
    highlighter = fzy_sorter.highlighter,
  })
end

function view_to_string(tabb)
  for k in pairs(tabb) do
	local v = tabb[k]
	if k == "anchor" then
	  local s = v:gsub("%.", " ")
	  s = s:gsub("^%l", string.upper)
	  return s
	elseif k == "search" then
	  return v
	elseif k == "page" then
	  return "p. " .. v
	else
	  return v
	end
  end
end

local function gen_data(span, note, backward)
  local arr = {}
  function insert_key(key, dir, view)
	  local target = utils.toml(span:cache_dir() .. key)

	  log.info(vim.inspect(view))
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

  if note == nil then
	-- parse global list of notes from ztl binary
	local result = vim.system({"ztl", "list-vim"}, { text = true }):wait()
	arr = vim.json.decode(result.stdout)
	table.remove(arr)
  elseif type(note) == "table" and type(note[1]) == "string" then
	  for _,v in pairs(note) do
		  insert_key(v, nil)
	  end
  elseif not backward then
	--note = utils.toml(span:cache_dir() .. note.target)
	for k,v in pairs(note["outgoing"]) do
		insert_key(v["target"], "outgoing", v["view"])
	end

	for k,v in pairs(note["children"]) do
		insert_key(v, "children")
	end
  else
	--note = utils.toml(span:cache_dir() .. note.target)
	for k,v in pairs(note["incoming"]) do
		insert_key(v, "incoming")
	end

	if note["parent"] ~= nil then
		insert_key(note["parent"], "parent")
	end
  end

  return arr
end

local function gen_finder(data) 
  local finder = finders.new_table {
	results = data,
	entry_maker = function(entry)
	  local res = vim.split(entry["target"] or ":", ":")

	  local function make_display(ent)
		if ent.value.dir == nil then
			displayer = entry_display.create {
			  separator = " ▏",
			  items = {
				{ width = 8 }, -- section
				{ remaining = true }, -- header
			  },
			}
		elseif ent.value.view ~= nil and next(ent.value.view) ~= nil then
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
				{ remaining = true }, -- header
			  },
			}
		  end

	    if ent.value.kind == nil then
	        ent.value.kind = "note"
	    end

		if ent.value.dir == nil then
		  return displayer {
		    -- text, highlight group
		    { ent.value.kind, "notePreviewKind" },
		    { ent.value.header, "notePreviewHeader" },
		  }
		end

		if ent.value.dir == "children" then
		 picto = ""
		elseif ent.value.dir == "parent" then
		 picto = ""
		elseif ent.value.dir == "outgoing" then
		 picto = "" 
		elseif ent.value.dir == "incoming" then
		 picto = ""
		end

		local display = {
		  -- text, highlight group
		  { ent.value.kind, "notePreviewKind" },
		  { " " .. picto .. "  ", "notePreviewPicto" },
		  { ent.value.header, "notePreviewHeader" },
		}
		if ent.value.view ~= nil and next(ent.value.view) ~= nil then
		  table.insert(display, {
			string.format("%20s", view_to_string(ent.value.view)), "markdownUrl"})
		end

		--log.info(vim.inspect(display))
		return displayer(display)
	  end

	  return {
		value = entry,
		display = make_display,
		ordinal = entry,
		filename = res[1],
		lnum = tonumber(res[2]),
	  }
	end,
  }

  return finder
end

function M.find_notes(span, note, backward, insert_mode)
	if type(note) == "table" and type(note[1]) == "string" then
	  prompt_title = "Selected Notes"
    elseif note ~= nil then
	  prompt_title = note["header"]
	  note = utils.toml(span:cache_dir() .. note.target)
	else
	  prompt_title = "All Notes"
	end
	
	local opts = {
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

	-- save (note, finder, last_prompt, last_position) entries
	local state = {
	  stack = {{nil, gen_finder(gen_data(span, note, backward)), "", 1}},
	  idx = 1
	}

	if note ~= nil then
	  state.stack[1][1] = note
	end

	-- create a Telescope picker; use vimgrep buffer for file preview
	pickers
	.new(opts, {
		finder = state.stack[1][2],
		sorter = post_sorter(opts),
		attach_mappings = function(prompt_bufnr, map)
		  if insert_mode == nil then
		    map('i', "<CR>", function() M.action_open(prompt_bufnr, span, state.stack[state.idx][1]) end)
		  elseif insert_mode.mode == "insert" then
		    map('i', "<CR>", function() M.action_open_insert(prompt_bufnr, span) end)
		  else
		    map('i', "<CR>", function() M.action_open_visual(prompt_bufnr, span, insert_mode.range) end)
		  end

		  map('i', "<C-f>", function() M.show_outgoing(prompt_bufnr, span, state) end)
		  map('i', "<C-b>", function() M.show_incoming(prompt_bufnr, span, state) end)
		  map('i', "<Right>", function() M.follow(prompt_bufnr, span, state) end)
		  map('i', "<Left>", function() M.retreat(prompt_bufnr, span, state) end)
			
		  return true
		end,
		previewer = previewers.vim_buffer_vimgrep:new(opts),
	})
	:find()
end

function M.action_open(bfnr, span, curr_note)
  local picker = actions_state.get_current_picker(bfnr)
  local selection = actions_state.get_selected_entry()
  if selection == nil then
	local prompt = picker:_get_prompt()
	picker:close_windows()

	-- only works for markdown for now
	if vim.bo.filetype ~= "markdown" then
	  return
	end

	if curr_note ~= nil then
	  vim.fn.cursor(tonumber(curr_note["span"]["start"]["line"]), 1)
	  local line = vim.fn.search("^#", "nw")
	  local content = vim.api.nvim_buf_get_lines(0, line - 1, line, false)[1]
	  local leading_hash = content:match("^#+")

	  vim.api.nvim_buf_set_lines(0, line-1, line-1, false, {leading_hash .. " " .. utils.string_random(6) .. " " .. prompt, "", "", ""})
	  vim.fn.cursor(line + 2, 1)
	  vim.schedule(function() vim.cmd("normal! zO") end)

	  return
	end

	  vim.api.nvim_buf_set_lines(0, -1, -1, false, {"# " .. utils.string_random(6) .. " " .. prompt, "", ""})
	  vim.cmd("normal G")

	  return
  end


  if selection.ordinal.note == nil then
	note = utils.toml(span:cache_dir() .. selection.ordinal.key)
  else
	note = selection.ordinal.note
  end

  if note.resource ~= nil then
    utils.open(span, note.resource, selection.ordinal.view or {})
  else
	picker:close_windows()
    utils.open(span, "key:" .. note.id, {})
  end
end

function M.action_open_insert(bfnr, span)
  local selection = actions_state.get_selected_entry()
  local picker = actions_state.get_current_picker(bfnr)

  picker:close_windows()

  vim.schedule(function()
	  local key = "[]("..selection.ordinal.key .. ")"
	  local row, col = unpack(vim.api.nvim_win_get_cursor(0))
	  vim.api.nvim_buf_set_text(0, row - 1, col, row - 1, col, { key .. " " })
	  vim.api.nvim_win_set_cursor(0, { row, col + 1 })
	  vim.cmd("startinsert")
  end)
end

function M.action_open_visual(bfnr, span, range)
  local selection = actions_state.get_selected_entry()
  local picker = actions_state.get_current_picker(bfnr)

  picker:close_windows()

  vim.schedule(function()
	  local key = selection.ordinal.key 
	  -- Extract the selected text
	  local lines = vim.api.nvim_buf_get_text(0, range[1][2] - 1, range[1][3] - 1, range[2][2] - 1, range[2][3], {})

	  local selected_text = table.concat(lines, "\n")
	  -- Surround the selected text with [](key)
	  local surrounded_text = string.format("[%s](%s)", selected_text, key)

	  -- Replace the selected text with the surrounded text
	  vim.api.nvim_buf_set_text(0, range[1][2] - 1, range[1][3] - 1, range[2][2] - 1, range[2][3], vim.split(surrounded_text, "\n"))
  end)
end

-- Follow current selected note
function M.follow(bfnr, span, state)
  local current_picker = actions_state.get_current_picker(bfnr)

  -- save current prompt for reuse, when we return
  state.stack[state.idx][3] = current_picker:_get_prompt()
  state.stack[state.idx][4] = current_picker:get_selection_row()

  local selection = actions_state.get_selected_entry()
  -- check if following note has same key
  if state.idx < #state.stack and state.stack[state.idx + 1][1].id ~= selection.ordinal.key then
	for i = #state.stack, state.idx + 1, -1 do
	  table.remove(state.stack, i)
	end
  end

  -- if next index exceeds number of finders, append
  -- one generated from selected entry
  state.idx = state.idx + 1
  if state.idx > #state.stack then
	local note = utils.toml(span:cache_dir() .. selection.ordinal.key)
	local finder = gen_finder(gen_data(span, note, false))

	table.insert(state.stack, {note, finder, "", 1})
  end

  --local callbacks = { unpack(current_picker._completion_callbacks) } -- shallow copy
  --vim.notify(vim.inspect(callbacks))
  --current_picker:register_completion_callback(function(self)
  --  vim.notify(state.stack[state.idx][4])
  --  self:set_selection(state.stack[state.idx][4])
  --  vim.notify(self:get_selection_row())
  --  self._completion_callbacks = callbacks
  --end)

  -- update new finder
  current_picker.prompt_border:change_title(state.stack[state.idx][1].header)
  current_picker:refresh(state.stack[state.idx][2])
  current_picker:set_prompt(state.stack[state.idx][3])
  --vim.notify(tostring(state.stack[state.idx][4]))
  --current_picker:set_selection(state.stack[state.idx][4])
  --vim.notify(tostring(current_picker:get_selection_row()))

  --current_picker:set_selection(state.stack[state.idx][4])
end

-- Retreat to previously selected note
function M.retreat(bfnr, span, state)
  local current_picker = actions_state.get_current_picker(bfnr)

  -- save current prompt for reuse, when we return
  if state.idx > 1 then
	state.idx = state.idx - 1
  end

  -- update new finder
  if state.stack[state.idx][1] ~= nil then
	current_picker.prompt_border:change_title(state.stack[state.idx][1].header)
  else
	current_picker.prompt_border:change_title("All Notes")
  end

  --vim.notify(tostring(state.stack[state.idx][4]))
  --current_picker:set_selection(state.stack[state.idx][4])
  --vim.notify(tostring(current_picker:get_selection_row()))

  current_picker:refresh(state.stack[state.idx][2])
  current_picker:set_prompt(state.stack[state.idx][3])
  vim.defer_fn(function() current_picker:set_selection(state.stack[state.idx][4]) end, 60)

  --local callbacks = { unpack(current_picker._completion_callbacks) } -- shallow copy
  --current_picker:register_completion_callback(function(self)
  --  --vim.notify(tostring(state.stack[state.idx][4]))
  --  --vim.notify(tostring(self:get_selection_row()))
  --  self._completion_callbacks = callbacks
  --end)

end

function M.show_outgoing(bfnr, span, state)
  -- if we are showing all notes, there is no distinction
  -- between incoming and outgoing links
  if state.stack[#state.stack][1] == nil then
	return
  end

  local note = utils.toml(span:cache_dir() .. state.stack[#state.stack][1].id)
  local finder = gen_finder(gen_data(span, note, false))

  -- replace last finder with new outgoing links
  state.stack[#state.stack][2] = finder

  local current_picker = actions_state.get_current_picker(bfnr)
  current_picker:refresh(finder)
end

function M.show_incoming(bfnr, span, state)
  -- if we are showing all notes, there is no distinction
  -- between incoming and outgoing links
  if state.stack[#state.stack][1] == nil then
	return
  end

  local note = utils.toml(span:cache_dir() .. state.stack[#state.stack][1].id)
  local finder = gen_finder(gen_data(span, note, true))

  -- replace last finder with new outgoing links
  state.stack[#state.stack][2] = finder

  local current_picker = actions_state.get_current_picker(bfnr)
  current_picker:refresh(finder)
end

function M.git_commits(span)
	-- TODO: support visual selection of multiple notes
	local notes = { span:note().target }

	-- get current file name
	local fname = vim.api.nvim_buf_get_name(0)
	local fname = Path:new(fname):make_relative(vim.fn.getcwd())

	local cmd = { "git", "log", "--pretty=oneline", "--abbrev-commit", "--no-patch" }
	for _, note in ipairs(notes) do
		table.insert(cmd, ".ztl/cache/" .. note)
	end
	
  local opts = {
	  entry_maker = make_entry.gen_from_git_commits(),
	  git_command = cmd,
		layout_strategy = "vertical",
		prompt_position = "top",
		layout_config = {
			height = {padding = 0},
			width = 0.75,
			preview_height = 0.6,
			preview_cutoff = 1,
			mirror = true,
		}
  }

  pickers
    .new(opts, {
      prompt_title = "Note Changes",
      finder = finders.new_oneshot_job(opts.git_command, opts),
      previewer = {
        --previewers.git_commit_message.new(opts),
		git_commit_msgs(opts),
      },
      sorter = conf.file_sorter(opts),
      attach_mappings = function(_, map)
        actions.select_default:replace(function(bnr) action_select_notes(bnr, span) end)
        return true
      end,
    })
    :find()
end

function action_select_notes(bufnr, span)
  local current_picker = actions_state.get_current_picker(bfnr)
  local selection = actions_state.get_selected_entry()

  local cmd = "git diff-tree --no-commit-id --name-only " .. selection.value .." -r -G'^hash = '"
  local result = vim.fn.systemlist(cmd)

  M.find_notes(span, result)
end

local ns_previewer = vim.api.nvim_create_namespace "telescope.previewers"

function git_commit_msgs(opts)
  local hl_map = {
    "TelescopeResultsIdentifier",
    "TelescopePreviewUser",
    "TelescopePreviewDate",
	"TelescopePreviewDirectory",
  }
  return previewers.new_buffer_previewer {
    title = "Git Message",
    get_buffer_by_name = function(_, entry)
      return entry.value
    end,

    define_preview = function(self, entry)
      local cmd = require("telescope.utils").__git_command({ "--no-pager", "log", "-n 1", "--stat", entry.value, "--", ':!.ztl' }, opts)

      putils.job_maker(cmd, self.state.bufnr, {
        value = entry.value,
        bufname = self.state.bufname,
        cwd = opts.cwd,
        callback = function(bufnr, content)
          if not content then
            return
          end
		  local pattern = "()|%s*()(%d+)()%s+()([%+]*)()([%-]*)()"

		  for k, v in ipairs(content) do
			local e1, s2, size, e2, s3, psigns, e3, msigns, e4 = v:match(pattern)
			if size ~= nil then
				vim.hl.range(bufnr, ns_previewer, "HtmlComment", {k-1, s2-1}, {k-1, e2-1})
				vim.hl.range(bufnr, ns_previewer, "GitSignsAdd", {k-1, s3-1}, {k-1, e3-1})
				vim.hl.range(bufnr, ns_previewer, "GitSignsDelete", {k-1, e3-1}, {k-1, e4})
			end
		  end

          for k, v in ipairs(hl_map) do
            local _, s = content[k]:find "%s"
            if s then
              vim.api.nvim_buf_add_highlight(bufnr, ns_previewer, v, k - 1, s, #content[k])
            end
          end
        end,
      })
    end,
  }
end

return M
