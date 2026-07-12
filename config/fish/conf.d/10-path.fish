for dir in "$HOME/.local/bin" "$HOME/.cargo/bin"
    if test -d "$dir"
        fish_add_path --global "$dir"
    end
end

set -l dotfiles_dir (__dotfiles_dir)
if test -n "$dotfiles_dir"
    set -gx DOTFILES_DIR "$dotfiles_dir"
end

for brew_bin in /opt/homebrew/bin/brew /usr/local/bin/brew /home/linuxbrew/.linuxbrew/bin/brew
    if test -x "$brew_bin"
        set -l brew_prefix (path dirname (path dirname "$brew_bin"))
        set -gx HOMEBREW_PREFIX "$brew_prefix"
        set -gx HOMEBREW_CELLAR "$brew_prefix/Cellar"
        set -gx HOMEBREW_REPOSITORY "$brew_prefix"
        fish_add_path --global --move --path "$brew_prefix/bin" "$brew_prefix/sbin"
        if test -n "$MANPATH[1]"
            set -gx MANPATH '' $MANPATH
        end
        if not contains "$brew_prefix/share/info" $INFOPATH
            set -gx INFOPATH "$brew_prefix/share/info" $INFOPATH
        end
        break
    end
end
