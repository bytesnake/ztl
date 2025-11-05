local M = {}

local defaults = {
	pdf_viewer = 'okular {file}',
	url_viewer = 'firefox {url}',
	telescope_mapping = {
	  ["<C-g>"] = require("ztl.telescope").action_open,
	  ["<C-f>"] = require("ztl.telescope").action_forward,
	  ["<C-b>"] = require("ztl.telescope").action_backward,
	},
	resources = {
	  mastodon = function(v) return "https://zettel.haus/@losch/" .. v end,
	  file = function(v) 
		-- check if file exists in directory
		local matches = vim.fn.glob(v, true, true)
		if #matches > 0 then
		  return vim.fs.normalize(v)
		else -- otherwise assume that this is a remote file
		  return "https://zettel.haus/source/" .. v
		end
	  end,
	  url = function(v) return v end,
	},
	viewer = {
	  pdf = function(url, view)
		local cmd = "ssh -o StrictHostKeyChecking=no losch@localhost -p 2020 'DISPLAY=:0.0 ~/.local/bin/cachepdf " .. url

		if view.anchor ~= nil then cmd = cmd .. "#" .. view.anchor end
		if view.page   ~= nil then cmd = cmd .. " --page " .. view.page end
		if view.search ~= nil then cmd = cmd .. " --find \"" .. view.search .. "\"" end
		return cmd .. "'"
	  end,
	  website = function(url, view)
		local cmd = "ssh -o StrictHostKeyChecking=no losch@localhost -p 2020 'DISPLAY=:0.0 firefox " .. url
		if view.anchor ~= nil then cmd = cmd .. "#" .. view.anchor end

		return cmd .. "'"
	  end,
	  markdown = function(url, view)
		require("ztl.utils").open_md_in_tabs(url)
	  end
	},
}

M.options = {}

function M.setup(opts)
    opts = opts or {}
    M.options = vim.tbl_deep_extend("force", defaults, opts)
end

M.setup()

return M

