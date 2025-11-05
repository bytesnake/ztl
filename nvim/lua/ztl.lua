local log = require "ztl.log"
local fncs = require "ztl.fncs"
local utils = require "ztl.utils"
local folding = require "ztl.folding"

-- setup shorcuts for commonly used vim functions
local ag = vim.api.nvim_create_augroup
local au = vim.api.nvim_create_autocmd

local current_fname = {}
local span = require("ztl.span"):new()

function switchFile()
  -- update span information with novel file
  local fname = vim.fn.expand("%")

  vim.cmd([[
  	syn region markdownLink matchgroup=markdownLinkDelimiter start="(" end=")" contains=markdownUrl keepend contained conceal
  	syn region markdownLinkText matchgroup=markdownLinkTextDelimiter start="!\=\[\%(\%(\_[^][]\|\[\_[^][]*\]\)*]\%( \=[[(]\)\)\@=" end="\]\%( \=[[(]\)\@=" nextgroup=markdownLink,markdownId skipwhite contains=@markdownInline,markdownLineStart concealends
  	set conceallevel=2
  	highlight markdownLinkText ctermfg=red guifg=#076678 cterm=bold term=bold gui=bold
  	set concealcursor=nc

    syn match rRefCommand "\\r" conceal nextgroup=rRefReference
    syn region rRefReference matchgroup=LineNr start="{" end="}" contains=rRefName conceal nextgroup=rRefName
    syn region rRefName matchgroup=LineNr start="{" end="}" concealends
    highlight link rRefName markdownLinkText
    highlight link rRefReference Macro
    highlight link rRefCommand LineNr
  ]])

  -- remember window ID for multiple opened windows
  local win_id = vim.api.nvim_get_current_win()

  if current_fname[win_id] == fname then
	return
  else
	current_fname[win_id] = fname
  end

  utils.call_and_watch(vim.fn.fnamemodify(fname, ":p"), function()
	local path = span:update_file(fname, win_id)

	-- override folding directives
	require("ztl.folding").setup(span.span, fname) 

	return path
  end, 100)
end

local buffer_number = -1

local M = {}

function M.setup(config)
  require("ztl.config").setup(config)

  -- check that the first file opened, was actually in a 
  -- ZTL subfolder, otherwise exit
  if span == nil then
	return
  end

  -- setup a autocmd group, and register our switch file callback
  local ztl_group = ag("ztl", { clear = true })
  au({"BufEnter", "WinNew"}, {
	  group = ztl_group,
	  pattern = { "*.md", "*.bib", "*.tex"},
	  callback = switchFile,
  })
  -- override span if working directory is changed
  au({"DirChanged"}, {
	  group = ztl_group,
	  pattern = {"global", "auto", "window", "tab"},
	  callback = function()
		  span = require("ztl.span"):new()
	  end
  })

  local cur_note = {}
  au({"CursorMoved"}, {
      group = ztl_group,
      pattern = { "*.md", "*.bib", "*.tex"},
      callback = function()
    	local note = span:note()
    	if note == nil then
    		return
    	end

    	if cur_note["target"] ~= note["target"] then
		  local target = span.wdir .. "cache/" .. note["target"] .. ".sixel.show"
		  local file = io.open(target, "w")
		  file.close()
    	end
    	cur_note = note
      end
  })

  require('lualine').setup({
  	sections = { 
  		lualine_b = {function() return span:current_target() end},
  		lualine_c = {function() return span:current_header() end},
  		lualine_x = {},
  		lualine_y = {},
  	},
  })
  
  vim.keymap.set("n", "gr", 
  	function() require("ztl.telescope").find_notes(span) end, { desc = "Global Note Preview" })

  vim.keymap.set("i", "<C-f>", 
  	function() require("ztl.telescope").find_notes(span, nil, nil, { mode = "insert" }) end, { desc = "Global Note Preview" })

  vim.keymap.set("v", "<C-f>", 
  	function() 
		local range = {vim.fn.getpos("v"), vim.fn.getpos(".") }

		require("ztl.telescope").find_notes(span, nil, nil, { mode = "visual", range = range } ) 
	end, { desc = "Global Note Preview" })

  vim.keymap.set("n", "gf",
    function() fncs.forward_follow(span) end, { desc = "Follow Link Forward" })

  vim.keymap.set("n", "gb",
    function() fncs.backward_follow(span) end, { desc = "Follow Link Backward" })
  vim.keymap.set("n", "gt", 
  	function() require("ztl.telescope").git_commits(span) end, { desc = "Git commits for note" })

  vim.keymap.set("n", "gs", 
  	function() require("ztl.telescope_sched").schedule(span) end, { desc = "Global schedule" })

  -- insert new note ID (random 6 digits character)
  vim.keymap.set('i', '<C-z>', function() 
      local uuid = utils.string_random(6)
      
      local row, col = unpack(vim.api.nvim_win_get_cursor(0))
      vim.api.nvim_buf_set_text(0, row - 1, col, row - 1, col, { uuid .. " " })
      vim.api.nvim_win_set_cursor(0, { row, col + 7 })
  end)

  vim.api.nvim_set_hl(0, "notePreviewKind", { fg = "#a31d1d" })
end

return M
