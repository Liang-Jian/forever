
#### homebrew 安装brew
```brew install mingw-w64```

#### install env
chakan env. 
echo $SHELL

vi .zchrc

abc@ubuntu ~ % cat .zchrc 
```
# ========= 基本环境变量配置（Apple Silicon） =========

# 设置 PATH（优先使用 Apple Silicon 默认的 Homebrew 路径）
export PATH="/opt/homebrew/bin:/opt/homebrew/sbin:$PATH"

# ========= Rust 交叉编译工具链（例如 x86_64-windows） =========

# Windows 交叉编译工具链（通过 Homebrew 安装 mingw-w64）
export CC_x86_64_pc_windows_gnu=x86_64-w64-mingw32-gcc
export CXX_x86_64_pc_windows_gnu=x86_64-w64-mingw32-g++
export AR_x86_64_pc_windows_gnu=x86_64-w64-mingw32-ar

# 如果你用的是 clang 作为默认编译器（可选）
# export CC=clang
# export CXX=clang++

# ========= Rust 设置 =========

# 加速 Rust 编译缓存（可选）
export CARGO_TERM_COLOR=always

# ========= Homebrew 国内源（可选） =========
# export HOMEBREW_BREW_GIT_REMOTE="https://mirrors.ustc.edu.cn/brew.git"
# export HOMEBREW_CORE_GIT_REMOTE="https://mirrors.ustc.edu.cn/homebrew-core.git"

# ========= 自动加载（生效） =========
if [ -f ~/.zshrc.local ]; then
  source ~/.zshrc.local
fi
```
source .zchrc

#### rust install target
rustup target list --installed
rustup target remove x86_64-pc-windows-gnu

#### build

cargo build --release --target x86_64-pc-windows-gnu