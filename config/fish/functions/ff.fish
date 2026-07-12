function ff --description 'select a file with fzf and bat preview'
    if not type -q fd; or not type -q fzf
        __gum_warn "ff: fd and fzf are required"
        return 127
    end

    fd --type f --hidden --follow --exclude .git |
        fzf --preview "$__fzf_file_preview" \
            --preview-window "right,50%,border-left" \
            --bind "ctrl-/:change-preview-window(down,60%,border-top|hidden|right,50%,border-left)" \
            $argv
end
