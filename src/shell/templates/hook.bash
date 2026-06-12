# gitid bash hook. Add to ~/.bashrc:  eval "$(gitid hook bash)"
_gitid_hook() {
  local _gitid_status=$?
  if [[ "${_GITID_PWD-}" != "$PWD" ]]; then
    _GITID_PWD=$PWD
    eval "$({{GITID}} env --shell bash)"
  fi
  return $_gitid_status
}
if [[ ";${PROMPT_COMMAND[*]:-};" != *";_gitid_hook;"* ]]; then
  if [[ "$(declare -p PROMPT_COMMAND 2>/dev/null)" == "declare -a"* ]]; then
    PROMPT_COMMAND+=(_gitid_hook)
  else
    PROMPT_COMMAND="_gitid_hook${PROMPT_COMMAND:+;$PROMPT_COMMAND}"
  fi
fi
_gitid_hook
