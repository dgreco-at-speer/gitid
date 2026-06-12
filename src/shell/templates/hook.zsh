# gitid zsh hook. Add to ~/.zshrc:  eval "$(gitid hook zsh)"
autoload -Uz add-zsh-hook
_gitid_hook() {
  eval "$({{GITID}} env --shell zsh)"
}
add-zsh-hook chpwd _gitid_hook
_gitid_hook
