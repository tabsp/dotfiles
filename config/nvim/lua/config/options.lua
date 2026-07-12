-- Options are automatically loaded before lazy.nvim startup

-- Prefer the repository root for pickers and explorers. Fall back to the LSP
-- root for files outside a Git repository.
vim.g.root_spec = { { ".git" }, "lsp", "cwd" }
