export GITID_COMPLETIONS_ACTIVE=1
# gitid zsh completions. Add to ~/.zshrc:  eval "$(gitid completions zsh)"
#compdef gitid
_gitid() {
  local -a subcmds
  subcmds=(add current dirs doctor edit env forget hook init list mcp remove setup show sync update use)
  local curcontext="$curcontext" state
  _arguments -C \
    '1: :->subcmd' \
    '2: :->arg1' \
    '3: :->arg2' \
    '*::arg:->rest'
  case $state in
    subcmd)
      compadd -a subcmds
      ;;
    arg1)
      case ${words[2]} in
        show|edit|remove|use)
          compadd -- $({{GITID}} __complete profiles "${words[CURRENT]}")
          ;;
        forget|current|doctor)
          _files -/
          ;;
      esac
      ;;
    arg2)
      case ${words[2]} in
        use)
          _files -/
          ;;
      esac
      ;;
  esac
}
compdef _gitid gitid
