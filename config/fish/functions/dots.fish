function dots --description 'open dotfiles config actions'
    if not type -q gum
        __gum_warn "dots: gum is required for the interactive menu"
        echo "Open dotfiles: cd "(string escape -- (__dotfiles_dir))
        return 127
    end

    set -l choice (printf '%s\n' \
        "Open dotfiles" "Edit fish config" "Edit nvim config" \
        "Reload fish config" "Dotman status" "Dotman sync" "Dotman plan" \
        "Sync fish plugins" "Update tldr pages" "Open lazygit" |
        gum filter --height 12 --placeholder "dotfiles action")
    test -n "$choice"; or return 0

    set -l editor "$EDITOR"
    test -n "$editor"; or set editor nvim
    set -l dir (__dotfiles_dir)

    switch "$choice"
        case "Open dotfiles"
            test -n "$dir"; or begin; __gum_warn "dotfiles directory not found"; return 1; end
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
            type -q fisher; or begin; __gum_warn "fisher is not installed"; return 127; end
            fisher update
        case "Update tldr pages"
            type -q tldr; or begin; __gum_warn "tldr is not installed"; return 127; end
            env TEALDEER_CONFIG_DIR="$HOME/.config/tealdeer" tldr --update
        case "Open lazygit"
            type -q lazygit; or begin; __gum_warn "lazygit is not installed"; return 127; end
            test -z "$dir"; or cd "$dir"
            lazygit
    end
end
