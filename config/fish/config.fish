if status is-interactive
    set fish_greeting
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

set -l workspace_dotfiles "$HOME/Workspace/dotfiles"
if test -d $workspace_dotfiles
    set -gx DOTFILES_DIR $workspace_dotfiles
end

for brew_bin in /opt/homebrew/bin/brew /usr/local/bin/brew /home/linuxbrew/.linuxbrew/bin/brew
    if test -x $brew_bin
        eval ($brew_bin shellenv)
        break
    end
end

function y
        set tmp (mktemp -t "yazi-cwd.XXXXXX")
        yazi $argv --cwd-file="$tmp"
        if set cwd (command cat -- "$tmp"); and [ -n "$cwd" ]; and [ "$cwd" != "$PWD" ]
                builtin cd -- "$cwd"
        end
        rm -f -- "$tmp"
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
    set -gx FZF_DEFAULT_COMMAND 'fd --type f --hidden --follow --exclude .git'
    set -gx FZF_CTRL_T_COMMAND $FZF_DEFAULT_COMMAND
    set -gx FZF_ALT_C_COMMAND 'fd --type d --hidden --follow --exclude .git'
    if type -q bat
        set -gx FZF_CTRL_T_OPTS "--preview 'bat --color=always --style=numbers --line-range=:500 {}'"
    end
    fzf --fish | source

    if type -q zoxide
        function zi --description 'jump to a zoxide directory with fzf'
            set -l dir (zoxide query -i)
            and cd $dir
        end
    end

    function ff --description 'select a file with fzf and bat preview'
        if not type -q fd
            echo "ff: fd is required" >&2
            return 127
        end

        set -l preview 'bat --color=always --style=numbers --line-range=:500 {}'
        if not type -q bat
            set preview 'sed -n "1,500p" {}'
        end

        fd --type f --hidden --follow --exclude .git |
            fzf --preview "$preview" $argv
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
    eval (try init | string collect)
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
            echo "Already inside tmux."
            return 0
        end

        tmux attach-session; or tmux new-session -s Work
    end
end

set -l local_fish_dir "$HOME/.config/fish/local.d"
if test -d $local_fish_dir
    for file in $local_fish_dir/*.fish
        source $file
    end
end
