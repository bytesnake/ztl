local M = {}
local function check_setup()
  return true
end

M.check = function()
  vim.health.start("ztl reportee")
  -- make sure setup function parameters are ok
  vim.health.info("informational information")
  vim.health.warn("something fishy ..")
  vim.health.error("oh oh")

  vim.health.start("ztl something")

  if not check_setup() then
    vim.health.ok("Setup is correct")
  else
    vim.health.error("Setup is incorrect")
  end
  -- do some more checking
  -- ...
end
return M
