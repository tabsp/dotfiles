-- n, v, i, t = mode names

local function termcodes(str)
  return vim.api.nvim_replace_termcodes(str, true, true, true)
end

local M = {}

M.general = {

  i = {
    -- go to  beginning and end
    ["<C-a>"] = { "<ESC>^i", "論 beginning of line" },
    ["<C-e>"] = { "<End>", "壟 end of line" },

    -- navigate within insert mode
    ["<C-h>"] = { "<Left>", "  move left" },
    ["<C-l>"] = { "<Right>", " move right" },
    ["<C-j>"] = { "<Down>", " move down" },
    ["<C-k>"] = { "<Up>", " move up" },

    ["jj"] = { "<ESC>", "esc" },
  },

  n = {

    ["<leader>rc"] = { ":e $MYVIMRC<CR>", "open init.lua" },
    ["<ESC>"] = { "<cmd> noh <CR>", "  no highlight" },

    -- switch between windows
    ["<C-h>"] = { "<C-w>h", " window left" },
    ["<C-l>"] = { "<C-w>l", " window right" },
    ["<C-j>"] = { "<C-w>j", " window down" },
    ["<C-k>"] = { "<C-w>k", " window up" },

    -- save
    ["S"] = { "<cmd> w <CR>", "﬚  save file" },

    -- Copy all
    ["<C-c>"] = { "<cmd> %y+ <CR>", "  copy whole file" },

    -- line numbers
    ["<leader>n"] = { "<cmd> set nu! <CR>", "   toggle line number" },
    ["<leader>rn"] = { "<cmd> set rnu! <CR>", "   toggle relative number" },

    ["Q"] = { "<cmd> q <CR>", "Quit" },

    ["H"] = { "^", "beginning of line" },
    ["L"] = { "$", "end of line" },
  },

  t = {
    ["<C-x>"] = { termcodes "<C-\\><C-N>", "   escape terminal mode" },
  },
}

M.nvimtree = {

  n = {
    -- toggle
    ["tt"] = { "<cmd> NvimTreeToggle <CR>", "   toggle nvimtree" },

    -- focus
    ["<leader>t"] = { "<cmd> NvimTreeFocus <CR>", "   focus nvimtree" },
  },
}

M.bufferline = {

  n = {
    ["<leader>="] = { "<cmd> BufferLineCycleNext <CR>", "-> buffer line next" },
    ["<leader>-"] = { "<cmd> BufferLineCyclePrev <CR>", "<- buffer line prev " },
  },
}

M.bufdelete = {
  n = {
    ["<C-d>"] = { "<cmd> Bdelete <CR>", "delete buffer" },
    ["<leader>x"] = { "<cmd> Bdelete <CR>", "delete buffer" },
  },
}

M.comment = {
  -- toggle comment in both modes
  n = {
    ["<leader>/"] = {
      function()
        require("Comment.api").toggle_current_linewise()
      end,

      "蘒  toggle comment",
    },
  },

  v = {
    ["<leader>/"] = {
      "<ESC><cmd>lua require('Comment.api').toggle_linewise_op(vim.fn.visualmode())<CR>",
      "蘒  toggle comment",
    },
  },
}

M.telescope = {
  n = {
    -- find
    ["<leader>ff"] = { "<cmd> Telescope find_files <CR>", "find files" },
    ["<leader>fa"] = { "<cmd> Telescope find_files follow=true no_ignore=true hidden=true <CR>", "find all" },
    ["<leader>fw"] = { "<cmd> Telescope live_grep <CR>", "live grep" },
    ["<leader>fb"] = { "<cmd> Telescope buffers <CR>", "find buffers" },
    ["<leader>fh"] = { "<cmd> Telescope help_tags <CR>", "help page" },
    ["<leader>fo"] = { "<cmd> Telescope oldfiles <CR>", "find oldfiles" },
    ["<leader>tk"] = { "<cmd> Telescope keymaps <CR>", "show keys" },
    ["<leader>fn"] = { "<cmd> Telescope noice <CR>", "show noices" },

    -- git
    ["<leader>cm"] = { "<cmd> Telescope git_commits <CR>", "   git commits" },
    ["<leader>gt"] = { "<cmd> Telescope git_status <CR>", "  git status" },

    -- projects
    ["<leader>fp"] = { "<cmd> Telescope projects <CR>", "find projects" },
  },
}

M.aerial = {
  n = {
    ["mm"] = { "<cmd> AerialToggle <CR>", "Aerial toggle" },
  },
}

M.glow = {
  n = {
    ["<leader>p"] = { "<cmd> Glow <CR>", "Preview markdown" },
  },
}

M.lazy = {
  n = {
    ["<leader>lz"] = { "<cmd> Lazy <CR>", "Manage plugins" },
  },
}

M.diffview = {
  n = {
    ["<leader>gd"] = { "<cmd> DiffviewOpen <CR>", "Open diff view" },
    ["<leader>gc"] = { "<cmd> DiffviewClose <CR>", "Close diff view" },
  },
}

return M
