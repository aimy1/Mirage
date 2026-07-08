#!/bin/bash
# Mirage TUI Audio Visualizer One-Click Installer for Linux
set -e

echo -e "\033[1;36m🌌 Welcome to Mirage Visualizer One-Click Installer\033[0m"
echo -e "\033[0;33mChecking system environment and installing dependencies...\033[0m"

# 1. 检测包管理器并安装 ALSA 编译依赖
if command -v apt-get &> /dev/null; then
    if ! dpkg -l | grep -q "^ii  libasound2-dev" || ! dpkg -l | grep -q "^ii  build-essential" &> /dev/null; then
        echo "Detected Debian/Ubuntu base system. Installing libasound2-dev and build-essential..."
        sudo apt-get update && sudo apt-get install -y libasound2-dev build-essential
    else
        echo "Dependencies (libasound2-dev, build-essential) are already installed."
    fi
elif command -v dnf &> /dev/null; then
    if ! rpm -q alsa-lib-devel gcc &> /dev/null; then
        echo "Detected Fedora/RHEL base system. Installing alsa-lib-devel..."
        sudo dnf install -y alsa-lib-devel gcc
    else
        echo "Dependencies (alsa-lib-devel, gcc) are already installed."
    fi
elif command -v pacman &> /dev/null; then
    if ! pacman -Qi alsa-lib &> /dev/null || ! pacman -Qi base-devel &> /dev/null; then
        echo "Detected Arch Linux base system. Installing alsa-lib..."
        sudo pacman -S --noconfirm alsa-lib base-devel
    else
        echo "Dependencies (alsa-lib, base-devel) are already installed."
    fi
else
    echo -e "\033[0;31mWarning: Unknown package manager. Please ensure ALSA development libraries and build-essential are installed.\033[0m"
fi

# 2. 检查并安装 Rust/Cargo 编译链
# 定义测速函数，返回连接耗时的毫秒数。如果超时或失败则返回 9999
get_delay_ms() {
    local url=$1
    local time_str
    if time_str=$(curl -o /dev/null -s -f -m 2.0 -w "%{time_total}" "$url"); then
        time_str=$(echo "$time_str" | tr ',' '.')
        local ms=$(awk -v t="$time_str" 'BEGIN { printf "%d\n", t * 1000 }')
        if [ -n "$ms" ]; then
            echo "$ms"
            return
        fi
    fi
    echo "9999"
}

echo "Testing connection speeds to official and mirror servers..."

# 1. 测速官方源 (拉取小文件)
DELAY_OFFICIAL=$(get_delay_ms "https://static.rust-lang.org/dist/channel-rust-stable.toml")
# 2. 测速 Rsproxy
DELAY_RSPROXY=$(get_delay_ms "https://rsproxy.cn")
# 3. 测速清华源
DELAY_TUNA=$(get_delay_ms "https://mirrors.tuna.tsinghua.edu.cn/rustup/")

echo "Results (ping latency):"
echo "  - Official Rust Server: ${DELAY_OFFICIAL}ms"
echo "  - Rsproxy Server:       ${DELAY_RSPROXY}ms"
echo "  - Tsinghua (TUNA) Server: ${DELAY_TUNA}ms"

# 默认选择官方
BEST_SOURCE="official"
MIN_DELAY=$DELAY_OFFICIAL

# 如果 Rsproxy 更快且正常
if [ "$DELAY_RSPROXY" -lt "$MIN_DELAY" ]; then
    MIN_DELAY=$DELAY_RSPROXY
    BEST_SOURCE="rsproxy"
fi

# 如果 清华源 更快且正常
if [ "$DELAY_TUNA" -lt "$MIN_DELAY" ]; then
    MIN_DELAY=$DELAY_TUNA
    BEST_SOURCE="tuna"
fi

# 如果官方源比 800ms 慢，且国内源中至少有一个可用
if [ "$DELAY_OFFICIAL" -gt 800 ] && [ "$MIN_DELAY" -lt 9999 ]; then
    # 强制不使用官方源，从国内源中选一个最快的
    if [ "$DELAY_RSPROXY" -le "$DELAY_TUNA" ]; then
        BEST_SOURCE="rsproxy"
    else
        BEST_SOURCE="tuna"
    fi
fi

USE_MIRROR=false
CONN_FAILED=false

# 应用选择的源并设置环境变量
if [ "$BEST_SOURCE" = "rsproxy" ]; then
    echo -e "\033[0;32mSelected high-speed mirror: Rsproxy (China)\033[0m"
    export RUSTUP_DIST_SERVER="https://rsproxy.cn"
    export RUSTUP_UPDATE_ROOT="https://rsproxy.cn/rustup"
    USE_MIRROR=true
elif [ "$BEST_SOURCE" = "tuna" ]; then
    echo -e "\033[0;32mSelected high-speed mirror: Tsinghua University TUNA (China)\033[0m"
    export RUSTUP_DIST_SERVER="https://mirrors.tuna.tsinghua.edu.cn/rustup"
    export RUSTUP_UPDATE_ROOT="https://mirrors.tuna.tsinghua.edu.cn/rustup"
    USE_MIRROR=true
else
    echo -e "\033[0;32mSelected: Official Rust Server\033[0m"
    if [ "$DELAY_OFFICIAL" -eq 9999 ]; then
        CONN_FAILED=true
    fi
fi

if [ "$CONN_FAILED" = true ]; then
    echo -e "\033[1;31m[WARNING] All connection tests timed out. Your machine might be offline or behind a restrictive firewall.\033[0m"
    if [ -z "$https_proxy" ] && [ -z "$http_proxy" ]; then
        echo -e "\033[1;33mIf you need a proxy to access external networks, please configure it in your terminal:\033[0m"
        echo -e "   export https_proxy=\"http://YOUR_PROXY_IP:PORT\""
        echo -e "   export http_proxy=\"http://YOUR_PROXY_IP:PORT\""
        echo ""
    fi
fi

if ! command -v cargo &> /dev/null; then
    echo "Rust/Cargo not found. Installing via rustup..."
    
    # 拉取安装脚本
    if ! curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y; then
        echo -e "\033[1;31mError: rustup installation failed.\033[0m"
        if [ "$CONN_FAILED" = true ]; then
            echo -e "\033[1;33mPlease configure a valid terminal proxy (HTTP/HTTPS) or verify your internet connection.\033[0m"
        fi
        exit 1
    fi
    # 将当前的 Cargo 路径写入会话环境
    source "$HOME/.cargo/env"
else
    echo "Rust environment detected: $(cargo --version)"
fi

# 3. 配置 Cargo 的 Crates 镜像以加速依赖包下载
if [ "$USE_MIRROR" = true ]; then
    mkdir -p "$HOME/.cargo"
    CARGO_CONFIG="$HOME/.cargo/config.toml"
    
    # 只要没有配过镜像，我们就写入
    if [ ! -f "$CARGO_CONFIG" ] || ! grep -q "replace-with" "$CARGO_CONFIG"; then
        # 备份已有的配置
        if [ -f "$CARGO_CONFIG" ]; then
            cp "$CARGO_CONFIG" "${CARGO_CONFIG}.bak"
        fi
        
        if [ "$BEST_SOURCE" = "rsproxy" ]; then
            echo "Setting up high-speed Cargo registry mirror: Rsproxy..."
            cat << 'EOF' > "$CARGO_CONFIG"
[source.crates-io]
replace-with = 'rsproxy-sparse'

[source.rsproxy-sparse]
registry = "sparse+https://rsproxy.cn/index/"

[net]
git-fetch-with-cli = true
EOF
        elif [ "$BEST_SOURCE" = "tuna" ]; then
            echo "Setting up high-speed Cargo registry mirror: Tsinghua TUNA..."
            cat << 'EOF' > "$CARGO_CONFIG"
[source.crates-io]
replace-with = 'tuna-sparse'

[source.tuna-sparse]
registry = "sparse+https://mirrors.tuna.tsinghua.edu.cn/rust-crates-adapter/crates.io-index/"

[net]
git-fetch-with-cli = true
EOF
        fi
        echo "Cargo registry mirror configured successfully."
    fi
fi

# 4. 编译并安装 Mirage
echo -e "\033[0;32mBuilding and installing Mirage...\033[0m"
cargo install --path .

# 4. 配置 Shell 环境变量
CARGO_BIN="$HOME/.cargo/bin"
if [[ ":$PATH:" != *":$CARGO_BIN:"* ]]; then
    echo -e "\033[0;33mConfiguring shell profile to add Mirage to PATH...\033[0m"
    
    # 自动识别默认 Shell
    SHELL_PROFILE=""
    if [ -n "$ZSH_VERSION" ] && [ -f "$HOME/.zshrc" ]; then
        SHELL_PROFILE="$HOME/.zshrc"
    elif [ -f "$HOME/.bashrc" ]; then
        SHELL_PROFILE="$HOME/.bashrc"
    fi

    if [ -n "$SHELL_PROFILE" ]; then
        echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> "$SHELL_PROFILE"
        echo -e "\033[1;32mMirage successfully installed! Please run 'source $SHELL_PROFILE' or reopen terminal.\033[0m"
    else
        echo -e "\033[0;33mPlease manually add '$HOME/.cargo/bin' to your PATH env.\033[0m"
    fi
else
    echo -e "\033[1;32mMirage successfully installed! You can now type 'mirage' in any terminal to launch it.\033[0m"
fi
