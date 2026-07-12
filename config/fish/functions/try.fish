function try --description 'create or enter a temporary workspace'
    set -l try_bin (type -P try)
    if test -z "$try_bin"
        __gum_warn "try: try-cli is required"
        return 127
    end

    set -l args $argv
    functions --erase try
    env SHELL=(command -v fish) command "$try_bin" init "$TRY_PATH" 2>/dev/null | string collect | source
    if functions -q try
        try $args
    else
        __gum_warn "try: initialization failed"
        return 1
    end
end
