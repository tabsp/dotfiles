function fish_title
    if set -q argv[1]
        echo -- (string sub -l 20 -- $argv[1]) (prompt_pwd -d 1 -D 1)
    else
        set -l command (status current-command)
        if test "$command" = fish
            set command
        end
        echo -- (string sub -l 20 -- $command) (prompt_pwd -d 1 -D 1)
    end
end
