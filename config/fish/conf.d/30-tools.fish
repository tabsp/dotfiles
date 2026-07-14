if test -f "$HOME/.config/mise/config.toml"
    set -gx MISE_TRUSTED_CONFIG_PATHS "$HOME/.config/mise/config.toml"
end

if type -q bat
    set -gx MANPAGER "sh -c 'col -bx | bat -l man -p'"
    set -gx MANROFFOPT -c
end
if type -q tldr
    set -gx TEALDEER_CONFIG_DIR "$HOME/.config/tealdeer"
end

set -gx TRY_PATH "$HOME/Workspace/tries"

# Hooks and prompts are only useful in an interactive shell. mise remains
# available to non-interactive shells through its shims without loading hooks.
if not status is-interactive
    if type -q mise
        mise activate fish --shims | source
    end
    return
end

if type -q zoxide
    zoxide init fish --cmd cd | source
end

if type -q mise
    mise activate fish | source
end

if type -q direnv
    direnv hook fish | source
end

if type -q starship
    starship init fish | source
end
