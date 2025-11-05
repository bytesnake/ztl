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

function M.schedule(span)
	local result = vim.system({"ztl", "schedule"}, { text = true }):wait()
	local arr = vim.json.decode(result.stdout)

	local opts = {
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

	local finder = finders.new_table {
	  results = arr,
	  entry_maker = function(entry)

		function make_display(ent)
		  displayer = entry_display.create {
			separator = "",
			items = {
			  { width = 6 }, -- label 
			  { width = 16 }, -- section
			  { remaining = true }, -- header
			},
		  }

		  if ent.value.state == "inactive" then
			return displayer {
			  { ent.value.label, "GruvboxGray" },
			  { "inactive", "GruvboxGray" },
			  { ent.value.header, "GruvboxGray" },
			}
		  else
			local pattern = "(%a+):(.*)"
			local state, timestamp = ent.value.state:match(pattern)

			if state == "due" then
			  state_disp = { "Due in " .. timestamp, "GruvboxGreen" }
			else
			  state_disp  = { "Overdue " .. timestamp, "GruvboxRed" }
			end
		  end

		  return displayer {
			{ ent.value.label, "notePreviewKind" },
			state_disp,
			{ ent.value.header, "notePreviewHeader" },
		  }
		end

		local note = utils.toml(span:cache_dir() .. entry.key)
		return {
		  value = entry,
		  display = make_display,
		  ordinal = entry,
		  filename = note.span.source,
		  lnum = tonumber(note.span.start.line),
		}
	  end
	}

  pickers
    .new(opts, {
      prompt_title = "Current Schedule",
      finder = finder,
      previewer = previewers.vim_buffer_vimgrep:new(opts),
      attach_mappings = function(_, map)
        actions.select_default:replace(function(bnr) action_select_notes(bnr, span) end)
        return true
      end,
    })
    :find()
end

return M
