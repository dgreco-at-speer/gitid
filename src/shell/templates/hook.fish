# gitid fish hook. Add to ~/.config/fish/config.fish:  gitid hook fish | source
set -gx GITID_HOOK_ACTIVE 1
function _gitid_hook --on-variable PWD --description 'gitid profile activation'
  {{GITID}} env --shell fish | source
end
_gitid_hook
