export PATH="/usr/local/bin:$PATH"
export EDITOR="nvim"
export GOPATH=$HOME/go
export PATH=$PATH:$GOPATH/bin
export LANGUAGE="en_US.UTF-8"
export LC_ALL="en_US.UTF-8"
export PATH=$PATH:/home/linuxbrew/.linuxbrew/bin

# >>> conda initialize >>>
# !! Contents within this block are managed by 'conda init' !!
__conda_setup="$($HOME/miniconda3/bin/conda shell.zsh hook 2> /dev/null)"

if [ $? -eq 0 ]; then
    eval "$__conda_setup"
else
    if [ -f "/usr/etc/profile.d/conda.sh" ]; then
        . "/usr/etc/profile.d/conda.sh"
    else
        export PATH="/usr/bin:$PATH"
    fi
fi
unset __conda_setup
# <<< conda initialize <<<
export CUDA_HOME=/usr/local/cuda-12
export PATH=${CUDA_HOME}/bin/${PATH:+:${PATH}}
export LD_LIBRARY_PATH=${CUDA_HOME}/lib64${LD_LIBRARY_PATH:+:${LD_LIBRARY_PATH}}
