local previewers = require('telescope.previewers')
local finders = require("telescope.finders")
local make_entry = require "telescope.make_entry"
local putils = require "telescope.previewers.utils"
local log = require "ztl.log"

local M = {}

function M.finder(source, opts)
	-- TODO: support visual selection of multiple notes
	local notes = { source }

	local cmd = { "git", "log", "--pretty=oneline", "--abbrev-commit", "--no-patch" }
	for _, note in ipairs(notes) do
		table.insert(cmd, ".ztl/notes/" .. note)
	end

	opts = {
	  cwd = opts.ctx.wdir, entry_maker = make_entry.gen_from_git_commits() }

	return finders.new_oneshot_job(cmd, opts)
end

function M.list_from_commit(commit)
  local cmd = "git diff-tree --no-commit-id --name-only " .. commit .." -r -G'^hash = '"

  local result = vim.iter(vim.fn.systemlist(cmd))
  result:map(function(v) return vim.fn.fnamemodify(v, ":t") end)
  result = result:totable()

  --require("ztl.log").info(vim.inspect(cmd))
  --require("ztl.log").info(vim.inspect(result))
  return result
end

local ns_previewer = vim.api.nvim_create_namespace "telescope.previewers"

function M.previewer(opts)
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
      local cmd = require("telescope.utils").__git_command({ "--no-pager", "log", "-n 1", "--stat", entry.value, "--", ':!.ztl' }, {})

      putils.job_maker(cmd, self.state.bufnr, {
        value = entry.value,
        bufname = self.state.bufname,
        cwd = opts.ctx.wdir,
        callback = function(bufnr, content)
          if not content or #content == 0 then
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
            if s ~= nil then
              vim.api.nvim_buf_add_highlight(bufnr, ns_previewer, v, k - 1, s, #content[k])
            end
          end
        end,
      })
    end,
  }
end

return M
