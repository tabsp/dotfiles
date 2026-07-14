if status is-interactive; and type -q fzf
    set -g __fzf_file_preview 'if test -d {}; eza --tree --level=2 --icons=always {} 2>/dev/null; or ls -la {}; else bat --color=always --style=numbers --line-range=:500 {} 2>/dev/null; or sed -n "1,500p" {}; end'
    set -g __fzf_dir_preview 'eza --tree --level=2 --icons=always {} 2>/dev/null; or ls -la {}'

    if type -q fd
        set -gx FZF_DEFAULT_COMMAND 'fd --type f --hidden --follow --exclude .git'
        set -gx FZF_CTRL_T_COMMAND 'fd --type f --type d --hidden --follow --exclude .git . $dir'
        set -gx FZF_ALT_C_COMMAND 'fd --type d --hidden --follow --exclude .git . $dir'
    end
    set -gx FZF_DEFAULT_OPTS '--height 80% --layout=reverse --border --info=inline'
    set -gx FZF_CTRL_T_OPTS "--preview '$__fzf_file_preview' --preview-window 'right,50%,border-left' --bind 'ctrl-/:change-preview-window(down,60%,border-top|hidden|right,50%,border-left)'"
    set -gx FZF_ALT_C_OPTS "--preview '$__fzf_dir_preview' --preview-window 'right,50%,border-left'"
    fzf --fish | source
end
