local utils = require "ztl.utils"

local Stack = {}
Stack.__index = Stack

function Stack.new(ctx, source, opts)
  local self = setmetatable({}, Stack)

  self.stack = {}
  self.idx = 1
  self.ctx = ctx

  self:push(source, opts)

  return self
end

local function string_or_array_of_strings(t)
	return type(t) == "string" or type(t) == "table"
end

function Stack:create_new(source, opts)
  vim.validate('source', source, string_or_array_of_strings,
  		true, 'key or array of keys')

  -- check if literature note, then forward shows attached resource
  local is_resource = false
  if type(source) == "string" then
    local current = utils.toml(opts.ctx:notes_dir() .. source)
	if current ~= nil then
	  is_resource = current.resource ~= nil
	end
  end

  if opts.mode == "resource" or (opts.mode == "forward" and is_resource) then
	return require("ztl.telescope.resource").finder(source, opts),
			require("ztl.telescope.resource").previewer(source, opts)
  elseif opts.mode == "forward" or opts.mode == "backward" then
	return require("ztl.telescope.links").finder(source, opts),
			require("ztl.telescope.links").previewer(opts)
  elseif opts.mode == "history" then
	return require("ztl.telescope.history").finder(source, opts),
			require("ztl.telescope.history").previewer(opts)
  end
end

function Stack:push(source, opts)
  local finder, previewer = self:create_new(source, opts)

  local current
  if type(source) == "string" then
    current = utils.toml(opts.ctx:notes_dir() .. source)
  end

  -- insert a new data source into the stack
  table.insert(self.stack, {
	current = current,
	finder = finder,
	previewer = previewer,
	last_prompt = "",
	last_pos = 1
  })
end

function Stack:current()
  return self.stack[self.idx]
end

function Stack:pop()
  self.idx = self.idx - 1
end

return Stack
