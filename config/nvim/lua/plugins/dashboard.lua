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
        { section = "startup" },
      }

      local keys = opts.dashboard.preset.keys
      if not keys then
        return
      end

      local session_index
      local has_session_picker = false
      for index, key in ipairs(keys) do
        if key.desc == "Projects (util.project)" then
          key.desc = "Projects"
          key.key = "p"
        end
        if key.key == "s" then
          session_index = index
        elseif key.key == "S" then
          has_session_picker = true
        end
      end

      if session_index and not has_session_picker then
        table.insert(keys, session_index + 1, {
          icon = " ",
          key = "S",
          desc = "Sessions",
          action = function()
            require("persistence").select()
          end,
        })
      end
    end,
  },
}
