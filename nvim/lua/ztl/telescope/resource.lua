local finders = require("telescope.finders")
local utils = require "ztl.utils"
local entry_display = require('telescope.pickers.entry_display')
local previewers = require('telescope.previewers')

local function process(data, id)
  local ok, res = pcall(vim.json.decode, data)
  if not ok then
	require("ztl.log").error("Could not get resource " .. id)
	--require("ztl.log").error(data)
  else
	local arr = {}
	for _, target in ipairs(res) do
	  table.insert(arr, {
		key = id,
		page = target["position"][1],
		header = target["target"],
		context = target["context"],
		view = { anchor = target["target"]},
		target_str = vim.json.encode(target),
	  })
	end

	return arr
  end
end

local Finder = {}
Finder.__index = Finder

function Finder:new(note, entry_maker)
  local stdin, stdout = vim.uv.new_pipe(), vim.uv.new_pipe()
  local id, resource = utils.unpack_resource(note.resource)
  local cmd = vim.uv.spawn("ztl-res", {
	args = {"--resource", resource},
	stdio = {stdin, stdout},
  })

  Finder.__call = function(t, ...)
    return t:_find(...)
  end

  Finder.close = function() end

  local obj = setmetatable({
	cmd = cmd,
	id = note.id,
	entry_maker = entry_maker,
	stdin = stdin,
	stdout = stdout,
  }, self)

  return obj
end

function Finder:_find(prompt, process_result, process_complete)
  --prompt = prompt:match("^search=(.*)$")

  local buf = ""
  function on_stdout(err, data)
	if data == nil or #data == 0 then
	  return
	end

	buf = buf .. data
	if #data == 65536 then
	  --require("ztl.log").info(#buf)
	  return
	end

	require("ztl.log").info(#buf)
	local arr = process(buf, self.id)
	local result_num = 0
	for _, elm in ipairs(arr) do
	  result_num = result_num + 1
	  local entry = self.entry_maker(elm)
	  if entry then
		entry.index = result_num
	  end
	  vim.schedule(function()
		process_result(entry) end)
	end

	process_complete()
	vim.uv.read_stop(self.stdout)
  end

  vim.uv.read_start(self.stdout, on_stdout)

  if string.find(prompt, "%s") then
	local pat = string.format("{ \"type\": \"Search\", \"text\": \"%s\"}", prompt)
	vim.uv.write(self.stdin, pat .. "\n")
  else
	vim.uv.write(self.stdin, '{"type": "Search"}\n')
  end
end

local M = {}

function M.finder(source, opts)
  local note = utils.toml(opts.ctx:notes_dir() .. source)
  if note == nil then
	return
  end

  local function make_display(entry)
	local displayer = entry_display.create {
	  separator = " ▏",
	  items = {
		{ width = 8 }, -- section
		{ remaining = true }, -- header
	  },
	}

	if entry.value["context"] ~= vim.NIL then
	  local inner = require("ztl.telescope.utils").getCenteredSubstring(entry.value.context[1]:gsub("\n", " "), entry.value.header, 50)

	  require("ztl.log").info(inner)

	  return displayer {
		-- text, highlight group
		{ "p. " .. entry.value.page, "notePreviewKind" },
		{ inner, "notePreviewHeader" },
	  }
	else
	  return displayer {
		-- text, highlight group
		{ "p. " .. entry.value.page, "notePreviewKind" },
		{ entry.value.header, "notePreviewHeader" },
	  }
	end
  end

  return Finder:new(note, function(entry)
	  return {
		value = entry,
		display = make_display,
		ordinal = entry,
	  }
	end)
end

local function render_resource(resource, width, height, zoom, stdout)
  -- round width and height to multiple of six (because of sixel)
  require("ztl.log").info(width, height)
  width, height = math.floor(width / 12) * 12, math.floor(height / 30) * 30
  require("ztl.log").info(width, height)
  return vim.system({"ztl-res", "--resource", resource, "--width", width, "--height", height, "--zoom", zoom}, {
	stdin = true,
	stdout = stdout,
	stderr = function(_, data)
	  if data ~= nil then
		require("ztl.log").info("Stderr " .. data)
	  end
	end,
  })
end

  -- https://github.com/nvim-telescope/telescope.nvim/blob/3a12a853ebf21ec1cce9a92290e3013f8ae75f02/lua/telescope/previewers/buffer_previewer.lua#L522
function M.previewer(source, opts)
	local note = utils.toml(opts.ctx:notes_dir() .. source)
	if note == nil then
	  return
	end

	local size = require("ztl.term").get_size()
	local _, resource = utils.unpack_resource(note.resource)

	local sequence = nil
	local function stdout(winid, data)
	  if data == nil then
		return
	  end

	  vim.schedule(function()
		--local curr_pos = vim.api.nvim_win_get_position(0)
		local pos = vim.api.nvim_win_get_position(winid)
		--local height = vim.api.nvim_win_get_height(bufnr)

		if sequence == nil then
		  sequence = "\x1b[s"
		  sequence = sequence .. "\x1b[" .. pos[1] + 1 .. ";" .. pos[2] + 1 .. "H"
		end

		sequence = sequence .. data

		if #data < 65536 then
		  sequence = sequence .. "\x1b[u"

		  vim.fn.chansend(vim.v.stderr, sequence)
		  vim.fn.chansend(vim.v.stderr, "")

		  sequence = nil
		end
	  end)
	end

	local cmd = nil
	local issue = require("ztl.utils").dejitter(function(elm)
		  cmd:write(string.format("{ \"type\": \"Render\", \"dest\": %s }\n", elm))
		end, 50)

	return previewers.new_buffer_previewer {
	  title = "Resource preview",
	  get_buffer_by_name = function(_, entry)
		return entry.value.key --.. "resource"
	  end,
	  define_preview = function(self, entry)
		if cmd == nil then
		  local winid = self.state.winid
		  local height = vim.api.nvim_win_get_height(winid)
		  local width = vim.api.nvim_win_get_width(winid)
		  cmd = render_resource(resource, width * size.cell_width, height * size.cell_height, 2, function(stderr, t) stdout(winid, t) end)
		end

		issue(entry.value.target_str)
	  end
	}
  end

  return M
