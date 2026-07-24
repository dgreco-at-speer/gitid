set -gx GITID_COMPLETIONS_ACTIVE 1
# gitid fish completions. Add to ~/.config/fish/config.fish:  gitid completions fish | source
function __gitid_at_pos
    test (count (commandline -opc)) -eq $argv[1]
end

complete -c gitid -f
complete -c gitid -n __fish_use_subcommand -a 'add current dirs doctor edit env forget hook init list mcp remove setup show sync update use'
complete -c gitid -n '__fish_seen_subcommand_from show edit remove' -a '({{GITID}} __complete profiles (commandline -ct))' -f
complete -c gitid -n '__fish_seen_subcommand_from use; and __gitid_at_pos 2' -a '({{GITID}} __complete profiles (commandline -ct))' -f
complete -c gitid -n '__fish_seen_subcommand_from use; and __gitid_at_pos 3' -a '(__fish_complete_directories)' -f
complete -c gitid -n '__fish_seen_subcommand_from forget current doctor' -a '(__fish_complete_directories)' -f
