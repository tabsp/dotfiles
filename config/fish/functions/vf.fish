function vf --description 'open a selected file in the default editor'
    set -l file (ff $argv)
    if test -n "$file"
        $EDITOR "$file"
    end
end
