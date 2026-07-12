-- Autocmds are automatically loaded on the VeryLazy event

-- LazyVim enables spell checking for text filetypes. Disable it for Markdown
-- after its FileType handlers have finished, while keeping <leader>us available
-- for explicitly turning spell checking back on.
local markdown_group = vim.api.nvim_create_augroup("user_markdown", { clear = true })

vim.api.nvim_create_autocmd("FileType", {
  group = markdown_group,
  pattern = { "markdown", "markdown.mdx" },
  callback = function(event)
    vim.schedule(function()
      if vim.api.nvim_buf_is_valid(event.buf) then
        for _, win in ipairs(vim.fn.win_findbuf(event.buf)) do
          vim.wo[win].spell = false
        end
      end
    end)
  end,
})
