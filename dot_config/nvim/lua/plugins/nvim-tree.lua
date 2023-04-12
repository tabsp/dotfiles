return {
  "nvim-tree/nvim-tree.lua",
  cmd = { "NvimTreeToggle" },
  opts = {
    disable_netrw = false,
    hijack_netrw = true,
    respect_buf_cwd = true,
    view = {
      number = false,
    },
    filters = {
      custom = { ".git" },
      dotfiles = false,
    },
    sync_root_with_cwd = true,
    update_focused_file = {
      enable = true,
      update_root = true,
    },
    actions = {
      open_file = {
        quit_on_open = true,
      },
    },
  },
}
