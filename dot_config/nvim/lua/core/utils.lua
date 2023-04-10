local M = {}

local merge_tb = vim.tbl_deep_extend

M.load_mappings = function(mappings, mapping_opt)
  local map_func

  map_func = function(keybind, mapping_info, opts)
    local mode = opts.mode
    opts.mode = nil
    vim.keymap.set(mode, keybind, mapping_info[1], opts)
  end

  mappings = mappings or vim.deepcopy(require "core.mappings")
  mappings.lspconfig = nil

  for _, section_mappings in pairs(mappings) do
    -- skip mapping this as its mapppings are loaded in lspconfig
    for mode, mode_mappings in pairs(section_mappings) do
      for keybind, mapping_info in pairs(mode_mappings) do
        local default_opts = merge_tb("force", { mode = mode }, mapping_opt or {})
        local opts = merge_tb("force", default_opts, mapping_info.opts or {})
        if mapping_info.opts then
          mapping_info.opts = nil
        end

        map_func(keybind, mapping_info, opts)
      end
    end
  end
end

local function default_on_open(term)
  vim.cmd "stopinsert"
  vim.api.nvim_buf_set_keymap(term.bufnr, "n", "q", "<cmd>close<CR>", { noremap = true, silent = true })
end

function M.open_term(cmd, opts)
  opts = opts or {}
  opts.size = opts.size or vim.o.columns * 0.5
  opts.direction = opts.direction or "float"
  opts.on_open = opts.on_open or default_on_open
  opts.on_exit = opts.on_exit or nil

  local Terminal = require("toggleterm.terminal").Terminal
  local new_term = Terminal:new {
    cmd = cmd,
    dir = "git_dir",
    auto_scroll = false,
    close_on_exit = false,
    start_in_insert = false,
    on_open = opts.on_open,
    on_exit = opts.on_exit,
  }
  new_term:open(opts.size, opts.direction)
end

return M
