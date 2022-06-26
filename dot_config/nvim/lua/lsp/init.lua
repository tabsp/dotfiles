local present1, lspconfig = pcall(require, "lspconfig")
local present2, lsp_installer = pcall(require, "nvim-lsp-installer")

if not present1 or not present2 then
  return
end

local utils = require "core.utils"

local servers = {
  sumneko_lua = require "lsp.lua",
  bashls = require "lsp.bashls",
}

local lsp_installer_options = {
  ui = {
    icons = {
      server_installed = " ",
      server_pending = " ",
      server_uninstalled = " ﮊ",
    },
    keymaps = {
      toggle_server_expand = "<CR>",
      install_server = "i",
      update_server = "u",
      check_server_version = "c",
      update_all_servers = "U",
      check_outdated_servers = "C",
      uninstall_server = "X",
    },
  },

  max_concurrent_installers = 10,
}

for name, _ in pairs(servers) do
  local server_available, server = lsp_installer.get_server(name)
  if server_available then
    if not server:is_installed() then
      vim.notify(string.format("please install [%s] LSP server", name), vim.log.levels.WARN)
    end
  end
end

lsp_installer.setup(lsp_installer_options)

local capabilities = vim.lsp.protocol.make_client_capabilities()

capabilities.textDocument.completion.completionItem = {
  documentationFormat = { "markdown", "plaintext" },
  snippetSupport = true,
  preselectSupport = true,
  insertReplaceSupport = true,
  labelDetailsSupport = true,
  deprecatedSupport = true,
  commitCharactersSupport = true,
  tagSupport = { valueSet = { 1 } },
  resolveSupport = {
    properties = {
      "documentation",
      "detail",
      "additionalTextEdits",
    },
  },
}

for name, m in pairs(servers) do
  if not m.config then
    m.config = {}
  end
  local opts = m.config
  if opts then
    opts.on_attach = function(client, bufnr)
      local lsp_mappings = require("core.mappings").lspconfig
      utils.load_mappings({ lsp_mappings }, { buffer = bufnr })
      if m.on_attach then
        m.on_attach(client, bufnr)
      end
    end
    opts.capabilities = capabilities
  end

  lspconfig[name].setup(opts)
end
