if status is-interactive
    # Commands to run in interactive sessions can go here
end

fish_add_path ~/.local/bin
if test (uname) = Darwin
    if test -d /opt/homebrew
        fish_add_path /opt/homebrew/bin /opt/homebrew/sbin
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
    zoxide init fish | source
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

alias z="cd"
if type -q lazygit
    alias lg="lazygit"
end

# opencode
fish_add_path \$HOME/.opencode/bin
