function y --description 'open yazi and change to its final directory'
    if not type -q yazi
        __gum_warn "y: yazi is required"
        return 127
    end

    set -l yazi_args $argv
    switch "$argv[1]"
        case --select -s
            if not type -q fd; or not type -q fzf
                __gum_warn "y --select: fd and fzf are required"
                return 127
            end
            set -l dir (fd --type d --hidden --follow --exclude .git . | fzf --preview "$__fzf_dir_preview" --preview-window "right,50%,border-left" $argv[2..])
            test -n "$dir"; or return 0
            set yazi_args "$dir"
        case --file -f
            set -l file (ff $argv[2..])
            test -n "$file"; or return 0
            set yazi_args (dirname "$file")
    end

    set -l tmp (mktemp -t "yazi-cwd.XXXXXX")
    command yazi $yazi_args --cwd-file="$tmp"
    if read -z cwd <"$tmp"; and test "$cwd" != "$PWD"; and test -d "$cwd"
        builtin cd -- "$cwd"
    end
    command rm -f -- "$tmp"
end
