function ls --description 'list files with eza icons when available'
    if type -q eza
        eza --icons=always $argv
    else
        command ls $argv
    end
end
