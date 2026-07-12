if status is-interactive
    set fish_greeting
end

# Machine-specific overrides intentionally load after all shared config.
set -l local_fish_dir "$HOME/.config/fish/local.d"
if test -d $local_fish_dir
    for file in $local_fish_dir/*.fish
        source $file
    end
end
