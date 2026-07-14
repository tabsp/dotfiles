local function started_with_files()
  for index = 0, vim.fn.argc() - 1 do
    local argument = vim.fn.argv(index)
    if argument ~= "" and vim.fn.isdirectory(argument) == 0 then
      return true
    end
  end
  return false
end

return {
  {
    "folke/persistence.nvim",
    init = function()
      local group = vim.api.nvim_create_augroup("user_persistence", { clear = true })

      -- Opening explicit files is treated as a one-off edit and must not replace
      -- the project session. Loading a session or changing project directories
      -- opts back into automatic saving.
      if started_with_files() then
        LazyVim.on_load("persistence.nvim", function()
          require("persistence").stop()
        end)
      end

      vim.api.nvim_create_autocmd("User", {
        group = group,
        pattern = "PersistenceLoadPost",
        callback = function()
          require("persistence").start()
        end,
      })

      vim.api.nvim_create_autocmd("DirChanged", {
        group = group,
        callback = function()
          if package.loaded.persistence then
            require("persistence").start()
          end
        end,
      })
    end,
  },
}
