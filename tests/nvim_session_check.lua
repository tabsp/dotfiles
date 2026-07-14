local persistence = require("persistence")
local mode = assert(vim.env.NVIM_SESSION_CHECK, "NVIM_SESSION_CHECK is required")

if mode == "direct" then
  assert(
    vim.wait(1000, function()
      return not persistence.active()
    end, 10),
    "direct file startup should not save a session"
  )
  persistence.fire("LoadPost")
  assert(persistence.active(), "loading a session should re-enable saving")
elseif mode == "dirchange" then
  assert(
    vim.wait(1000, function()
      return not persistence.active()
    end, 10),
    "direct file startup should begin with saving disabled"
  )
  vim.cmd.cd(vim.fn.fnamemodify(vim.fn.tempname(), ":h"))
  assert(persistence.active(), "changing directories should re-enable saving")
elseif mode == "directory" then
  assert(persistence.active(), "directory startup should keep session saving enabled")
else
  error("unknown NVIM_SESSION_CHECK mode: " .. mode)
end

persistence.stop()
print("Neovim session assertion passed: " .. mode)
