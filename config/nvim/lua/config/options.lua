-- Options are automatically loaded before lazy.nvim startup

-- Prefer the repository root for pickers and explorers. Fall back to the LSP
-- root for files outside a Git repository.
vim.g.root_spec = { { ".git" }, "lsp", "cwd" }

-- No configured plugin uses Neovim's legacy remote-plugin providers. Disable
-- their runtime probes; standalone Node-based tools such as markdown-preview
-- are unaffected.
vim.g.loaded_node_provider = 0
vim.g.loaded_python3_provider = 0
vim.g.loaded_ruby_provider = 0
vim.g.loaded_perl_provider = 0
