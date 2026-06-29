local g = vim.g

-- Use OSC 52 in tmux for clipboard sync
if vim.env.TMUX then
  g.clipboard = "osc52"
  vim.env.TMUX = nil
end
