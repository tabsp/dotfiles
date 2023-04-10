require "core.options"
require "core.lazy"

vim.defer_fn(function()
  require("core.utils").load_mappings()
end, 0)
require "core.autocmds"
