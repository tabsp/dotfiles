local force_inactive = {
  filetypes = {
    "NvimTree",
    "packer",
    "fugitive",
    "fugitiveblame",
    "qf",
    "help",
    "Trouble",
    "terminal",
  },
  buftypes = {
    "terminal",
    "nofile",
  },
  bufnames = {},
}

-- local function get_color(hlgroup, attr)
--   return vim.fn.synIDattr(vim.fn.synIDtrans(vim.fn.hlID(hlgroup)), attr, "gui")
-- end

local function copy_update_component(component, updates)
  local new_component = {}
  for k, v in pairs(component) do
    new_component[k] = v
  end
  for k, v in pairs(updates) do
    new_component[k] = v
  end
  return new_component
end

local tokyonight_colors = {
  black = "#15161E",
  bg = "#16161d",
  fg = "#c8c093",
  brightbg = "#3b4261",

  red = "#f7768e",
  orange = "#ff9e64",
  orange2 = "#db4b4b",
  yellow = "#e0af68",
  green = "#9ece6a",
  aqua = "#73daca",
  lightblue = "#b4f9f8",
  teal = "#2ac3de",
  cyan = "#7dcfff",
  blue = "#7aa2f7",
  blue1 = "#3d59a1",
  blue2 = "#33467C",
  magenta = "#bb9af7",
  purple = "#9d7cd8",
  gray1 = "#c0caf5",
  gray2 = "#a9b1d6",
  gray3 = "#9aa5ce",
  white = "#cfc9c2",
  gray4 = "#565f89",
  gray5 = "#414868",
  bg_highlight = "#292e42",
  bg1 = "#24283b",
  bg2 = "#1a1b26",
  diag_error = "#e82424",
  diag_warn = "#ff9e3b",
  diag_hint = "#6a9589",
  diag_info = "#658594",
  vcs_add = "#76946a",
  vcs_del = "#c34043",
  vcs_change = "#dca561",
}

local colors = tokyonight_colors

local function has_buftype(winid)
  winid = winid or 0
  local bufnr = vim.api.nvim_win_get_buf(winid)
  if vim.bo[bufnr].buftype == "" then
    return false
  else
    return true
  end
end

local function hide_in_width(winid)
  winid = winid or 0
  return vim.fn.winwidth(winid) >= 100
end

local function is_focused()
  return vim.api.nvim_get_current_buf() == tonumber(vim.g.actual_curbuf)
end

local ViMode = {
  provider = {
    name = "vi_mode",
    opts = {
      show_mode_name = true,
    },
  },
  hl = function()
    return {
      name = require("feline.providers.vi_mode").get_mode_highlight_name(),
      fg = require("feline.providers.vi_mode").get_mode_color(),
      style = "bold",
    }
  end,
  right_sep = " ",
  left_sep = " ",
}

local FileSize = {
  enabled = hide_in_width,
  provider = "file_size",
  right_sep = " ",
}

local FileName = {
  enabled = function(winid)
    return not has_buftype(winid)
  end,
  provider = function(component)
    local filename = vim.api.nvim_buf_get_name(0)
    local path = vim.fn.fnamemodify(filename, ":~:.")
    path = path == "" and "[No Name]" or path
    if (not hide_in_width()) or (#path > 0.29 * vim.fn.winwidth(0)) then
      path = vim.fn.pathshorten(path)
    end

    local extension = vim.fn.fnamemodify(filename, ":e")
    local icon_str, icon_color = require("nvim-web-devicons").get_icon_color(filename, extension, { default = true })
    local icon = { str = icon_str .. " ", hl = { fg = icon_color } }

    component.right_sep = {}
    if vim.bo.modified then
      table.insert(component.right_sep, { str = "[+]", hl = { fg = "green", name = "FelineFileFlagModified" } })
    end
    if vim.bo.readonly then
      table.insert(component.right_sep, { str = " ", hl = { fg = "red", name = "FelineFileFlagRO" } })
    end
    table.insert(component.right_sep, "%<  ")
    return path, icon
  end,
  hl = function()
    if vim.bo.modified then
      return { fg = "magenta", style = "bold", name = "FelineFileNameModified" }
    else
      return { fg = "purple", style = "bold", name = "FelineFileName" }
    end
  end,
}

local HelpFilename = {
  enabled = function()
    return vim.bo.filetype == "help"
  end,
  provider = function()
    local filename = vim.api.nvim_buf_get_name(0)
    return vim.fn.fnamemodify(filename, ":t")
  end,
  hl = { fg = "blue" },
}

local GitBranch = {
  provider = "git_branch",
  hl = { fg = "yellow" },
  right_sep = " ",
}
local DiffAdd = {
  provider = "git_diff_added",
  icon = "+",
  right_sep = " ",
  hl = { fg = "vcs_add" },
}
local DiffDel = {
  provider = "git_diff_removed",
  icon = "-",
  right_sep = " ",
  hl = { fg = "vcs_del" },
}
local DiffChange = {
  provider = "git_diff_changed",
  icon = "~",
  right_sep = " ",
  hl = { fg = "vcs_change" },
}

local DiagSep = {
  provider = "  ",
  enabled = function()
    return next(vim.lsp.buf_get_clients(0)) ~= nil
  end,
}

local DiagErr = {
  provider = "diagnostic_errors",
  hl = { fg = "diag_error" },
  right_sep = " ",
}
local DiagWarn = {
  provider = "diagnostic_warnings",
  hl = { fg = "diag_warn" },
  right_sep = " ",
}
local DiagHint = {
  provider = "diagnostic_hints",
  hl = { fg = "diag_hint" },
  right_sep = " ",
}
local DiagInfo = {
  provider = "diagnostic_info",
  hl = { fg = "diag_info" },
  right_sep = " ",
}

local LSPMessages = {
  enabled = function()
    return next(vim.lsp.buf_get_clients(0)) ~= nil
  end,
  provider = function()
    return require("lsp-status").status()
  end,
  hl = { fg = "gray4" },
  right_sep = " ",
}

local TSMessages = {
  provider = function()
    return require("nvim-treesitter").statusline { indicator_size = 50 } or ""
  end,
  hl = { fg = "gray4" },
  right_sep = " ",
}

local FileType = {
  enabled = function()
    return hide_in_width() or has_buftype()
  end,
  provider = "file_type",
  hl = { fg = "blue", style = "bold" },
  right_sep = " ",
}

local InactiveFileType = copy_update_component(FileType, {
  enabled = has_buftype,
  right_sep = { str = " %q", hl = { fg = "blue" } },
})

local FileFormat = {
  enabled = function()
    return vim.bo.fileformat ~= "unix" and hide_in_width()
  end,
  provider = function()
    return vim.bo.fileformat:upper()
  end,
  hl = { fg = "green" },
  right_sep = " ",
}
local FileEncoding = {
  enabled = function()
    local enc = (vim.bo.fenc ~= "" and vim.bo.fenc) or vim.o.enc
    return enc ~= "utf-8" and hide_in_width()
  end,
  provider = function()
    local enc = (vim.bo.fenc ~= "" and vim.bo.fenc) or vim.o.enc
    return enc:upper()
  end,
  hl = { fg = "green" },
  right_sep = " ",
}
local Position = {
  provider = "position",
}
local Percent = {
  provider = "line_percentage",
  left_sep = " ",
  hl = {
    style = "bold",
  },
}
local ScrollBar = {
  provider = "scroll_bar",
  left_sep = " ",
  hl = {
    fg = "blue",
    style = "bold",
  },
}

local function null_ls_providers(filetype)
  local registered = {}
  local sources_avail, sources = pcall(require, "null-ls.sources")
  if sources_avail then
    for _, source in ipairs(sources.get_available(filetype)) do
      for method in pairs(source.methods) do
        registered[method] = registered[method] or {}
        table.insert(registered[method], source.name)
      end
    end
  end
  return registered
end

local function null_ls_sources(filetype, source)
  local methods_avail, methods = pcall(require, "null-ls.methods")
  return methods_avail and null_ls_providers(filetype)[methods.internal[source]] or {}
end

local LSPActive = {
  enabled = function()
    return next(vim.lsp.buf_get_clients(0)) ~= nil
  end,
  icon = " ",
  provider = function()
    local buf_client_names = {}
    for _, client in pairs(vim.lsp.buf_get_clients(0)) do
      if client.name == "null-ls" then
        vim.list_extend(buf_client_names, null_ls_sources(vim.bo.filetype, "FORMATTING"))
        vim.list_extend(buf_client_names, null_ls_sources(vim.bo.filetype, "DIAGNOSTICS"))
      else
        table.insert(buf_client_names, client.name)
      end
    end
    return table.concat(buf_client_names, ", ")
  end,
  hl = { fg = "green", style = "bold" },
  right_sep = " ",
}

local WorkDir = {
  provider = function()
    local icon = (vim.fn.haslocaldir(0) == 1 and "l" or "g") .. " " .. " "
    local cwd = vim.fn.getcwd(0)
    cwd = vim.fn.fnamemodify(cwd, ":~")
    if (not hide_in_width()) or (#cwd > 0.22 * vim.fn.winwidth(0)) then
      cwd = vim.fn.pathshorten(cwd)
    end
    return cwd .. "/", icon
  end,
  right_sep = " ",
  hl = { fg = "blue1" },
}

local TerminalName = {
  enabled = function()
    return vim.bo.buftype == "terminal"
  end,
  icon = " ", -- 
  provider = function()
    local tname, _ = vim.api.nvim_buf_get_name(0):gsub(".*:", "")
    return tname
  end,
  hl = { fg = "purple", style = "bold" },
}

local TerminalMode = copy_update_component(ViMode, {
  enabled = function()
    local terminals = { "terminal", "dap-repl" }
    return vim.fn.index(terminals, vim.bo.filetype) ~= -1 and is_focused()
  end,
})

local Spell = {
  enabled = function()
    return vim.wo.spell
  end,
  provider = "SPELL",
  hl = { fg = "yellow", style = "bold" },
  right_sep = " ",
}

local Gps = {
  enabled = function()
    return require("nvim-gps").is_available()
  end,
  provider = function()
    return require("nvim-gps").get_location()
  end,
  hl = { fg = "gray4" },
}

local components = {
  active = {
    {
      ViMode,
      WorkDir,
      FileName,
      GitBranch,
      DiffAdd,
      DiffDel,
      DiffChange,
      DiagSep,
      DiagErr,
      DiagWarn,
      DiagHint,
      DiagInfo,
    },
    {
      LSPMessages,
      Gps,
      TSMessages,
    },
    {
      LSPActive,
      Spell,
      FileSize,
      FileType,
      FileFormat,
      FileEncoding,
      Position,
      Percent,
      ScrollBar,
    },
  },
  inactive = { { TerminalMode, InactiveFileType, HelpFilename, TerminalName, FileName }, {} },
}

require("feline").setup {
  theme = colors,
  components = components,
  force_inactive = force_inactive,
}
