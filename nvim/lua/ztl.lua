local utils = require "ztl.utils"
local ZtlCtx = require("ztl.context").ZtlCtx

-- setup shorcuts for commonly used vim functions
local ag = vim.api.nvim_create_augroup
local au = vim.api.nvim_create_autocmd

local watch = nil
local function switchFile()
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

  local ctx = ZtlCtx.new(fname)
  if ctx == nil then
	  return
  end

  utils.call_and_watch(ctx.source, function()
	ctx:update()
    vim.w.ztl_span = ctx.span

	require("ztl.log").info(vim.inspect(ctx.span))
    -- override folding directives
    require("ztl.folding").setup()
  end, 100)
end

local M = {}

M.Ctx = ZtlCtx

function M.setup(config)
  require("ztl.config").setup(config)

  -- setup a autocmd group, and register our switch file callback
  local ztl_group = ag("ztl", { clear = true })
  au({"BufEnter", "WinNew"}, {
	  group = ztl_group,
	  pattern = { "*.md", "*.bib", "*.tex"},
	  callback = switchFile,
  })

  local cur_note = {}
  au({"CursorMoved"}, {
      group = ztl_group,
      pattern = { "*.md", "*.bib", "*.tex"},
      callback = function()
		local ctx = ZtlCtx.current()
		if ctx == nil then
			return
		end

    	local note = ctx:note()
    	if note == nil then
    		return
    	end

    	if cur_note["target"] ~= note["target"] then
		  local target = ZtlCtx.current().wdir .. "/.ztl/cache/" .. note["target"] .. ".sixel.show"
		  local file = io.open(target, "w")
		  if file ~= nil then
			  file:close()
		  end
    	end
    	cur_note = note
      end
  })

  require('lualine').setup({
  	sections = {
  		lualine_b = {function() return ZtlCtx.current():current_target() end},
  		lualine_c = {function() return ZtlCtx.current():current_header() end},
  		lualine_x = {},
  		lualine_y = {},
  	},
  })

  -- setup `<Plug>` keymaps
  require("ztl.keymaps").setup()

  -- call setup function of configuration
  if require("ztl.config").options.setup() then
	  -- special functions with location depending behaviour
	  vim.keymap.set("n", "gf", "<Plug>ZtlFollow")
	  vim.keymap.set("n", "gb", "<Plug>ZtlRetreat")
	  vim.keymap.set("n", "gh", "<Plug>ZtlHistory")
	  vim.keymap.set("n", "go", "<Plug>ZtlOpenResource")
	  vim.keymap.set("i", "<C-z>", "<Plug>ZtlInsertKey")

	  -- these are functionalities directly supported by the API
	  vim.keymap.set("n", "gs", require("ztl.fncs").schedule)
	  vim.keymap.set("n", "gr", require("ztl.fncs").find_notes)
	  vim.keymap.set("i", "<C-f>", function()
		  require("ztl.fncs").find_notes(nil, { action = "insert" }) end)
	  vim.keymap.set("v", "<C-f>", function()
		  require("ztl.fncs").find_notes(nil, { action = "visual" }) end)

  end

  -- setup highlight groups
  vim.api.nvim_set_hl(0, "notePreviewKind", { fg = "#a31d1d" })
end

return M
