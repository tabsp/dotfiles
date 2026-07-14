local function contains(values, expected)
  return type(values) == "table" and vim.tbl_contains(values, expected)
end

local function check(condition, message)
  if not condition then
    error("nvim config check failed: " .. message)
  end
end

for _, provider in ipairs({ "node", "perl", "python3", "ruby" }) do
  check(vim.g["loaded_" .. provider .. "_provider"] == 0, provider .. " provider should be disabled")
end

local conform = LazyVim.opts("conform.nvim")
check(contains(conform.formatters_by_ft.fish, "fish_indent"), "fish_indent formatter is not configured")
check(contains(conform.formatters_by_ft.markdown, "prettier"), "Prettier is not configured for Markdown")
check(contains(conform.formatters_by_ft.markdown, "markdownlint-cli2"), "markdownlint-cli2 is not configured")
check(contains(conform.formatters_by_ft.markdown, "markdown-toc"), "markdown-toc is not configured")

local lint = LazyVim.opts("nvim-lint")
check(contains(lint.linters_by_ft.fish, "fish"), "Fish syntax linter is not configured")
check(contains(lint.linters_by_ft.markdown, "markdownlint-cli2"), "Markdown linter is not configured")
local markdown_linter = lint.linters["markdownlint-cli2"]
check(type(markdown_linter.condition) == "function", "Markdown linter condition is not configured")
check(not markdown_linter.condition({ filename = "/tmp/CLAUDE.md" }), "CLAUDE.md should skip Markdown linting")
check(markdown_linter.condition({ filename = "/tmp/README.md" }), "regular Markdown files should be linted")

local mason = LazyVim.opts("mason.nvim")
for _, tool in ipairs({ "prettier", "markdownlint-cli2", "markdown-toc" }) do
  check(contains(mason.ensure_installed, tool), tool .. " is not managed by Mason")
end

local project = LazyVim.opts("project.nvim")
check(project.manual_mode == true, "project.nvim should use manual mode")
check(project.scope_chdir == "tab", "project.nvim should scope cwd changes to the current tab")

local project_key = vim.fn.maparg("<leader>fp", "n", false, true)
check(not vim.tbl_isempty(project_key), "project picker keymap is missing")

local dashboard = LazyVim.opts("snacks.nvim").dashboard
local dashboard_project_key
local dashboard_session_key
for _, key in ipairs(dashboard.preset.keys or {}) do
  if key.desc == "Projects" then
    dashboard_project_key = key.key
  elseif key.desc == "Sessions" then
    dashboard_session_key = key.key
  end
end
check(dashboard_project_key == "p", "Dashboard project key should be lowercase p")
check(dashboard_session_key == "S", "Dashboard session picker key should be uppercase S")

check(require("lazy.core.config").options.checker.enabled == false, "lazy.nvim background checker should be disabled")

local lsp = LazyVim.opts("nvim-lspconfig")
check(lsp.servers.gopls ~= nil, "Go LSP is not configured")
check(lsp.servers.marksman ~= nil, "Markdown LSP is not configured")
check(require("lazy.core.config").plugins.rustaceanvim ~= nil, "Rust tooling is not configured")

print("Neovim configuration assertions passed")
