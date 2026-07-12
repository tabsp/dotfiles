function __gum_info --argument-names message
    if status is-interactive; and type -q gum
        gum style --foreground "#89b4fa" --bold -- "$message"
    else
        echo "$message"
    end
end
