local map = vim.keymap.set

-- Insert
map("i", "<C-a>", "<ESC>^i",  { desc = "beginning of line" })
map("i", "<C-e>", "<End>",    { desc = "end of line" })
map("i", "<C-h>", "<Left>",   { desc = "move left" })
map("i", "<C-l>", "<Right>",  { desc = "move right" })
map("i", "<C-j>", "<Down>",   { desc = "move down" })
map("i", "<C-k>", "<Up>",     { desc = "move up" })
map("i", "jj",    "<ESC>",    { desc = "esc" })

-- Normal
map("n", "<C-h>",     "<C-w>h",              { desc = "window left" })
map("n", "<C-l>",     "<C-w>l",              { desc = "window right" })
map("n", "<C-j>",     "<C-w>j",              { desc = "window down" })
map("n", "<C-k>",     "<C-w>k",              { desc = "window up" })
map("n", "<leader>w", "<cmd> w <CR>",        { desc = "save file" })
map("n", "<C-c>",     "<cmd> %y+ <CR>",      { desc = "copy whole file" })
map("n", "Q",         "<cmd> q <CR>",        { desc = "quit" })
map("n", "H",         "^",                   { desc = "beginning of line" })
map("n", "L",         "$",                   { desc = "end of line" })
map("n", "<ESC>",     "<cmd> noh <CR>",      { desc = "no highlight" })
map("n", "<leader>rc", "<cmd> e $MYVIMRC<CR>", { desc = "open init.lua" })
