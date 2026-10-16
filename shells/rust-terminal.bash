# Preserve login-shell setup, then report cwd at each prompt.
[[ -f /etc/profile ]] && source /etc/profile
if [[ -f ~/.bash_profile ]]; then source ~/.bash_profile
elif [[ -f ~/.bash_login ]]; then source ~/.bash_login
elif [[ -f ~/.profile ]]; then source ~/.profile
elif [[ -f ~/.bashrc ]]; then source ~/.bashrc
fi
_rust_terminal_cwd() { printf '\033]0;RUST_TERMINAL_CWD:%s\007' "$(pwd -W)"; }
if [[ $(declare -p PROMPT_COMMAND 2>/dev/null) == 'declare -a '* ]]; then
    PROMPT_COMMAND+=(_rust_terminal_cwd)
else
    PROMPT_COMMAND="${PROMPT_COMMAND:+$PROMPT_COMMAND; }_rust_terminal_cwd"
fi
