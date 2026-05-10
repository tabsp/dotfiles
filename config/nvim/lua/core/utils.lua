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

return M
