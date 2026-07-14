if status is-interactive
    set fish_greeting
end

# Machine-specific overrides intentionally load after all shared config and live
# outside the repository so host-only settings cannot be committed by mistake.
set -l local_fish_dir "$HOME/.config/fish-local"
if test -d "$local_fish_dir"
    for file in "$local_fish_dir"/*.fish
        source "$file"
    end
end
