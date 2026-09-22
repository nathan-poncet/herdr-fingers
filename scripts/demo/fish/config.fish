set -g fish_greeting
set -gx GIT_PAGER cat
set -gx PAGER cat
set -gx CLICOLOR 1
set -gx TERM xterm-256color

function fish_prompt
    set_color cyan
    echo -n (basename (pwd))
    set_color normal
    echo -n ' '
    set_color green
    echo -n '❯ '
    set_color normal
end

function fish_right_prompt
end

set -g fish_autosuggestion_enabled 0
