# gitid fish hook. Add to ~/.config/fish/config.fish:  gitid hook fish | source
function _gitid_hook --on-variable PWD --description 'gitid profile activation'
  {{GITID}} env --shell fish | source
end
_gitid_hook
