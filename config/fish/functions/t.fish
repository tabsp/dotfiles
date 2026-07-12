function t --description 'attach to tmux, or create the Work session'
    if not type -q tmux
        __gum_warn "t: tmux is required"
        return 127
    end
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
