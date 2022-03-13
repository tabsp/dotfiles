alias pon="export https_proxy=http://127.0.0.1:7890 http_proxy=http://127.0.0.1:7890 all_proxy=socks5://127.0.0.1:7890"
alias poff="unset http_proxy;unset https_proxy;unset all_proxy;"
alias lg="lazygit"
alias rg="ranger"
alias vi="nvim"
alias vim="nvim"
alias cnpm="npm --registry=https://registry.npm.taobao.org \
    --cache=$HOME/.npm/.cache/cnpm \
    --disturl=https://npm.taobao.org/dist \
    --userconfig=$HOME/.cnpmrc"
