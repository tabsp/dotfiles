local function project_dirs()
  local projects = {}
  local seen = {}

  local function add(dir)
    if dir and dir ~= "" and not seen[dir] and vim.fn.isdirectory(dir) == 1 then
      seen[dir] = true
      table.insert(projects, dir)
    end
  end

  local historyfile = vim.fn.stdpath("data") .. "/project_nvim/project_history"
  local file = io.open(historyfile, "r")
  if file then
    for line in file:lines() do
      add(line)
    end
    file:close()
  end

  return projects
end

return {
  {
    "folke/snacks.nvim",
    opts = function(_, opts)
      opts.dashboard = opts.dashboard or {}
      opts.dashboard.preset = opts.dashboard.preset or {}

      opts.dashboard.preset.header = [[
██████╗  ██████╗ ████████╗███████╗
██╔══██╗██╔═══██╗╚══██╔══╝██╔════╝
██║  ██║██║   ██║   ██║   ███████╗
██║  ██║██║   ██║   ██║   ╚════██║
██████╔╝╚██████╔╝   ██║   ███████║
╚═════╝  ╚═════╝    ╚═╝   ╚══════╝]]

      opts.dashboard.sections = {
        { section = "header" },
        { icon = " ", title = "Keymaps", section = "keys", indent = 2, padding = 1 },
        {
          icon = " ",
          title = "Projects",
          section = "projects",
          dirs = project_dirs,
          indent = 2,
          padding = 1,
        },
        { section = "startup" },
      }

      local keys = opts.dashboard.preset.keys
      if not keys then
        return
      end

      for _, key in ipairs(keys) do
        if key.desc == "Projects (util.project)" then
          key.desc = "Projects"
        end
      end
    end,
  },
}
