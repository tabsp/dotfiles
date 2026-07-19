-- Keymaps are automatically loaded on the VeryLazy event
local map = vim.keymap.set

-- Insert
map("i", "<C-a>", "<ESC>^i", { desc = "beginning of line" })
map("i", "<C-e>", "<End>", { desc = "end of line" })
map("i", "<C-l>", "<Right>", { desc = "move right" })
map("i", "<C-j>", "<Down>", { desc = "move down" })
map("i", "jj", "<ESC>", { desc = "esc" })

-- Navigation
map({ "n", "x", "o" }, "gh", "0", { desc = "Go to line start" })
map({ "n", "x", "o" }, "gl", "$", { desc = "Go to line end" })

-- Normal
map("n", "<M-h>", "<cmd>vertical resize -2<cr>", { desc = "Decrease window width" })
map("n", "<M-j>", "<cmd>resize -2<cr>", { desc = "Decrease window height" })
map("n", "<M-k>", "<cmd>resize +2<cr>", { desc = "Increase window height" })
map("n", "<M-l>", "<cmd>vertical resize +2<cr>", { desc = "Increase window width" })
