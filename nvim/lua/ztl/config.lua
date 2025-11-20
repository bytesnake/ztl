local M = {}

local defaults = {
	setup = function() return true end,
	--telescope_mapping = {
	--  ["<C-g>"] = require("ztl.telescope").action_open,
	--  ["<C-f>"] = require("ztl.telescope").action_forward,
	--  ["<C-b>"] = require("ztl.telescope").action_backward,
	--},
	resources = {
	  mastodon = function(v) return v end,
	  file = function(v) return v end,
	  url = function(v) return v end,
	},
	viewer = { },
}

M.options = {}

function M.setup(opts)
    opts = opts or {}
    M.options = vim.tbl_deep_extend("force", defaults, opts)
end

M.setup()

return M

