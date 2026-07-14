function ssh-compat --description 'SSH with a broadly supported terminal type'
    if not type -q ssh
        __gum_warn "ssh-compat: ssh is required"
        return 127
    end

    set -lx TERM xterm-256color
    command ssh $argv
end
