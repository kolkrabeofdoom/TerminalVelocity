# Take over the session key before anything else runs, and keep it out of the
# environment so child processes and login scripts never see it.
_terminalvelocity_token=${TERMINALVELOCITY_CWD_TOKEN-}
unset TERMINALVELOCITY_CWD_TOKEN
# Preserve login-shell setup, then report cwd at each prompt.
[[ -f /etc/profile ]] && source /etc/profile
if [[ -f ~/.bash_profile ]]; then source ~/.bash_profile
elif [[ -f ~/.bash_login ]]; then source ~/.bash_login
elif [[ -f ~/.profile ]]; then source ~/.profile
elif [[ -f ~/.bashrc ]]; then source ~/.bashrc
fi
_terminalvelocity_cwd() {
    if [[ -n $_terminalvelocity_token ]]; then
        printf '\033]0;TERMINALVELOCITY_CWD:%s:%s\007' "$_terminalvelocity_token" "$(pwd -W)"
    fi
}
if [[ $(declare -p PROMPT_COMMAND 2>/dev/null) == 'declare -a '* ]]; then
    PROMPT_COMMAND+=(_terminalvelocity_cwd)
else
    PROMPT_COMMAND="${PROMPT_COMMAND:+$PROMPT_COMMAND; }_terminalvelocity_cwd"
fi
