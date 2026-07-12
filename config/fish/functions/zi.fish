function zi --description 'jump to a zoxide directory with fzf'
    if not type -q zoxide
        __gum_warn "zi: zoxide is required"
        return 127
    end
    set -l dir (zoxide query -i)
    and cd "$dir"
end
