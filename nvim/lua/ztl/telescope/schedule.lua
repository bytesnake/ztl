local pickers = require("telescope.pickers")
local finders = require("telescope.finders")
local previewers = require("telescope.previewers")
local entry_display = require('telescope.pickers.entry_display')
local actions = require('telescope.actions')
local utils = require("ztl.utils")

local M = {}

function M.schedule(span)
	local result = vim.system({"ztl", "--format", "json", "schedule"}, { text = true }):wait()
	local arr = vim.json.decode(result.stdout)
  if arr["Err"] ~= nil then
	  for k in pairs(arr["Err"]) do
		  vim.notify(table.concat(arr["Err"][k], "\n"), "error")
	  end

	  return
  end

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
	  results = arr.Ok.Schedule,
	  entry_maker = function(entry)

		local function make_display(ent)
		  local displayer = entry_display.create {
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

			local state_disp
			if state == "due" then
			  state_disp = { "Due in " .. timestamp, "GruvboxGreen" }
			else
			  state_disp  = { "Overdue " .. timestamp, "GruvboxRed" }
			end

			return displayer {
			  { ent.value.label, "notePreviewKind" },
			  state_disp,
			  { ent.value.header, "notePreviewHeader" },
			}
		  end
		end

		local note = utils.toml(span:notes_dir() .. entry.key)
		if note == nil then
		  return nil
		end

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
