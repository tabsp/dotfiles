if status is-interactive
    set fish_greeting
end

# gum Catppuccin Mocha theme
set -gx GUM_INPUT_CURSOR_FOREGROUND "#89b4fa"
set -gx GUM_INPUT_PROMPT_FOREGROUND "#89b4fa"
set -gx GUM_FILTER_INDICATOR_FOREGROUND "#a6e3a1"
set -gx GUM_FILTER_MATCH_FOREGROUND "#f5c2e7"
set -gx GUM_FILTER_PROMPT_FOREGROUND "#89b4fa"
set -gx GUM_CHOOSE_CURSOR_FOREGROUND "#f9e2af"
set -gx GUM_CHOOSE_SELECTED_FOREGROUND "#f9e2af"
set -gx GUM_SPIN_SPINNER_FOREGROUND "#94e2d5"
set -gx GUM_SPIN_TITLE_FOREGROUND "#94e2d5"
# confirm: high-contrast buttons
set -gx GUM_CONFIRM_PROMPT_FOREGROUND "#89b4fa"
set -gx GUM_CONFIRM_SELECTED_FOREGROUND 0
set -gx GUM_CONFIRM_SELECTED_BACKGROUND 2
set -gx GUM_CONFIRM_UNSELECTED_FOREGROUND 7
set -gx GUM_CONFIRM_UNSELECTED_BACKGROUND 0

set -gx LANG en_US.UTF-8
set -gx LC_ALL en_US.UTF-8

function __gum_info --argument-names message
    if status is-interactive; and type -q gum
        gum style --foreground "#89b4fa" --bold -- "$message"
    else
        echo "$message"
    end
end

function __gum_warn --argument-names message
    if status is-interactive; and type -q gum
        gum style --foreground "#f9e2af" --bold -- "$message" >&2
    else
        echo "$message" >&2
    end
end

function __dotfiles_dir
    set -l candidates
    if set -q DOTFILES_DIR
        set -a candidates "$DOTFILES_DIR"
    end
    set -a candidates "$HOME/.local/share/dotman/repos/main" "$HOME/Workspace/dotfiles"

    for dir in $candidates
        if test -d "$dir"
            echo "$dir"
            return 0
        end
    end

    return 1
end

set -l local_bin "$HOME/.local/bin"
if test -d $local_bin
    fish_add_path $local_bin
end

set -l cargo_bin "$HOME/.cargo/bin"
if test -d $cargo_bin
    fish_add_path $cargo_bin
end

set -gx GEM_HOME "$HOME/.local/share/gem"

set -l dotfiles_dir (__dotfiles_dir)
if test -n "$dotfiles_dir"
    set -gx DOTFILES_DIR "$dotfiles_dir"
end

for brew_bin in /opt/homebrew/bin/brew /usr/local/bin/brew /home/linuxbrew/.linuxbrew/bin/brew
    if test -x $brew_bin
        eval ($brew_bin shellenv fish)
        break
    end
end

if type -q nvim
    set -gx EDITOR nvim
    set -gx VISUAL nvim
else if type -q vim
    set -gx EDITOR vim
    set -gx VISUAL vim
else
    set -gx EDITOR vi
    set -gx VISUAL vi
end

function y
    if not type -q yazi
        __gum_warn "y: yazi is required"
        return 127
    end

    set -l yazi_args $argv
    switch "$argv[1]"
        case --select -s
            if not type -q fd
                __gum_warn "y --select: fd is required"
                return 127
            end
            if not type -q fzf
                __gum_warn "y --select: fzf is required"
                return 127
            end

            set -l dir (fd --type d --hidden --follow --exclude .git . | fzf --preview "$__fzf_dir_preview" --preview-window "right,50%,border-left" $argv[2..])
            if test -z "$dir"
                return 0
            end
            set yazi_args "$dir"
        case --file -f
            if not functions -q ff
                __gum_warn "y --file: ff is required"
                return 127
            end

            set -l file (ff $argv[2..])
            if test -z "$file"
                return 0
            end
            set yazi_args (dirname "$file")
    end

    set -l tmp (mktemp -t "yazi-cwd.XXXXXX")
    command yazi $yazi_args --cwd-file="$tmp"
    if read -z cwd <"$tmp"; and test "$cwd" != "$PWD"; and test -d "$cwd"
        builtin cd -- "$cwd"
    end
    command rm -f -- "$tmp"
end

function ys --description 'select a directory with fzf and open yazi'
    y --select $argv
end

function yf --description 'select a file with fzf and open its directory in yazi'
    y --file $argv
end

if type -q zoxide
    zoxide init fish --cmd cd | source
end

if type -q direnv
    direnv hook fish | source
end

if type -q mise
    mise activate fish | source
end

if type -q fzf
    set -g __fzf_file_preview 'if test -d {}; eza --tree --level=2 --icons=always {} 2>/dev/null; or ls -la {}; else bat --color=always --style=numbers --line-range=:500 {} 2>/dev/null; or sed -n "1,500p" {}; end'
    set -g __fzf_dir_preview 'eza --tree --level=2 --icons=always {} 2>/dev/null; or ls -la {}'

    set -gx FZF_DEFAULT_COMMAND 'fd --type f --hidden --follow --exclude .git'
    set -gx FZF_CTRL_T_COMMAND 'fd --type f --type d --hidden --follow --exclude .git . $dir'
    set -gx FZF_ALT_C_COMMAND 'fd --type d --hidden --follow --exclude .git . $dir'
    set -gx FZF_DEFAULT_OPTS '--height 80% --layout=reverse --border --info=inline'
    set -gx FZF_CTRL_T_OPTS "--preview '$__fzf_file_preview' --preview-window 'right,50%,border-left' --bind 'ctrl-/:change-preview-window(down,60%,border-top|hidden|right,50%,border-left)'"
    set -gx FZF_ALT_C_OPTS "--preview '$__fzf_dir_preview' --preview-window 'right,50%,border-left'"
    fzf --fish | source

    if type -q zoxide
        function zi --description 'jump to a zoxide directory with fzf'
            set -l dir (zoxide query -i)
            and cd $dir
        end
    end

    function ff --description 'select a file with fzf and bat preview'
        if not type -q fd
            __gum_warn "ff: fd is required"
            return 127
        end

        fd --type f --hidden --follow --exclude .git |
            fzf --preview "$__fzf_file_preview" \
                --preview-window "right,50%,border-left" \
                --bind "ctrl-/:change-preview-window(down,60%,border-top|hidden|right,50%,border-left)" \
                $argv
    end

    function vf --description 'open a selected file in the default editor'
        set -l file (ff $argv)
        if test -z "$file"
            return 0
        end

        $EDITOR "$file"
    end
end

if type -q bat
    set -gx MANPAGER "sh -c 'col -bx | bat -l man -p'"
    set -gx MANROFFOPT -c
end

if type -q tldr
    set -gx TEALDEER_CONFIG_DIR "$HOME/.config/tealdeer"
end

if type -q try
    set -gx TRY_PATH "$HOME/Workspace/tries"
    env SHELL=(command -v fish) try init "$TRY_PATH" 2>/dev/null | string collect | source
end

if type -q starship
    starship init fish | source
end

if type -q nvim
    alias vim=nvim
end

function ls --description 'alias ls=eza --icons=always'
    if type -q eza
        eza --icons=always $argv
    else
        command ls $argv
    end
end

if type -q lazygit
    alias lg="lazygit"
end

if type -q tmux
    function t --description 'attach to tmux, or create the Work session'
        if set -q TMUX
            __gum_info "Already inside tmux."
            return 0
        end

        set -l sessions (tmux list-sessions -F '#S' 2>/dev/null)
        if test (count $sessions) -eq 0
            tmux new-session -s Work
            return
        end

        if test (count $sessions) -eq 1; or not type -q gum
            tmux attach-session; or tmux new-session -s Work
            return
        end

        set -l target (printf '%s\n' $sessions | gum filter --height 12 --placeholder "tmux session")
        if test -n "$target"
            tmux attach-session -t "$target"
        end
    end
end

function dots --description 'open dotfiles config actions'
    if not type -q gum
        __gum_warn "dots: gum is required for the interactive menu"
        echo "Open dotfiles: cd "(string escape -- (__dotfiles_dir))
        echo "Edit fish:     \$EDITOR ~/.config/fish/config.fish"
        echo "Reload fish:   source ~/.config/fish/config.fish"
        return 127
    end

    set -l choice (printf '%s\n' \
        "Open dotfiles" \
        "Edit fish config" \
        "Edit nvim config" \
        "Reload fish config" \
        "Dotman status" \
        "Dotman sync" \
        "Dotman plan" \
        "Sync fish plugins" \
        "Update tldr pages" \
        "Open lazygit" | gum filter --height 12 --placeholder "dotfiles action")

    if test -z "$choice"
        return 0
    end

    set -l editor "$EDITOR"
    if test -z "$editor"
        set editor nvim
    end

    set -l dir (__dotfiles_dir)

    switch "$choice"
        case "Open dotfiles"
            if test -z "$dir"
                __gum_warn "dotfiles directory not found"
                return 1
            end
            cd "$dir"
        case "Edit fish config"
            $editor "$HOME/.config/fish/config.fish"
        case "Edit nvim config"
            $editor "$HOME/.config/nvim"
        case "Reload fish config"
            source "$HOME/.config/fish/config.fish"
            __gum_info "fish config reloaded"
        case "Dotman status"
            dotman status
        case "Dotman sync"
            dotman sync
        case "Dotman plan"
            dotman plan --headless
        case "Sync fish plugins"
            if not type -q fisher
                __gum_warn "fisher is not installed"
                return 127
            end
            fisher update
        case "Update tldr pages"
            if not type -q tldr
                __gum_warn "tldr is not installed"
                return 127
            end
            env TEALDEER_CONFIG_DIR="$HOME/.config/tealdeer" tldr --update
        case "Open lazygit"
            if not type -q lazygit
                __gum_warn "lazygit is not installed"
                return 127
            end
            if test -n "$dir"
                cd "$dir"
            end
            lazygit
    end
end

set -l local_fish_dir "$HOME/.config/fish/local.d"
if test -d $local_fish_dir
    for file in $local_fish_dir/*.fish
        source $file
    end
end
