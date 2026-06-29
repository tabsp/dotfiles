local map = vim.keymap.set

-- Insert
map("i", "<C-a>", "<ESC>^i", { desc = "beginning of line" })
map("i", "<C-e>", "<End>", { desc = "end of line" })
map("i", "<C-l>", "<Right>", { desc = "move right" })
map("i", "<C-j>", "<Down>", { desc = "move down" })
map("i", "jj", "<ESC>", { desc = "esc" })

-- Normal
map("n", "<C-c>", "<cmd> %y+ <CR>", { desc = "copy whole file" })
map("n", "<leader>rc", "<cmd> e $MYVIMRC<CR>", { desc = "open init.lua" })
