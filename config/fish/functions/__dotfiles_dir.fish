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
