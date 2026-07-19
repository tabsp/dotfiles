function del --description 'move files to the system trash'
    if test (count $argv) -eq 0
        __gum_warn 'del: provide at least one path'
        return 2
    end

    if type -q trash-put
        command trash-put -- $argv
        return $status
    end

    # Homebrew keeps trash-cli keg-only on macOS because its command names can
    # shadow system utilities, so resolve its executable without changing PATH.
    if test (uname -s) = Darwin; and type -q brew
        set -l trash_put (brew --prefix trash-cli 2>/dev/null)/bin/trash-put
        if test -x "$trash_put"
            command "$trash_put" -- $argv
            return $status
        end
    end

    __gum_warn 'del: trash-cli is not installed'
    return 127
end
