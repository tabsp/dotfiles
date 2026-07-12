function __gum_warn --argument-names message
    if status is-interactive; and type -q gum
        gum style --foreground "#f9e2af" --bold -- "$message" >&2
    else
        echo "$message" >&2
    end
end
