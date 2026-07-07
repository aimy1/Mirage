#!/bin/bash
# Mirage TUI Audio Visualizer One-Click Installer for Linux
set -e

echo -e "\033[1;36m🌌 Welcome to Mirage Visualizer One-Click Installer\033[0m"
echo -e "\033[0;33mChecking system environment and installing dependencies...\033[0m"

# 1. 检测包管理器并安装 ALSA 编译依赖
if command -v apt-get &> /dev/null; then
    echo "Detected Debian/Ubuntu base system. Installing libasound2-dev..."
    sudo apt-get update && sudo apt-get install -y libasound2-dev build-essential
elif command -v dnf &> /dev/null; then
    echo "Detected Fedora/RHEL base system. Installing alsa-lib-devel..."
    sudo dnf install -y alsa-lib-devel gcc
elif command -v pacman &> /dev/null; then
    echo "Detected Arch Linux base system. Installing alsa-lib..."
    sudo pacman -S --noconfirm alsa-lib base-devel
else
    echo -e "\033[0;31mWarning: Unknown package manager. Please ensure ALSA development libraries and build-essential are installed.\033[0m"
fi

# 2. 检查并安装 Rust/Cargo 编译链
# 智能测速：拉取 300 字节的 stable 配置文件。如果耗时超过 0.8 秒或连接超时，说明下载大文件会极慢，自动启用镜像。
echo "Testing connection speed to official Rust servers..."
USE_MIRROR=false

TIME_STR=$(curl -o /dev/null -s -m 2.5 -w "%{time_total}" https://static.rust-lang.org/dist/channel-rust-stable.toml || echo "9.9")

if [ "$TIME_STR" = "9.9" ]; then
    USE_MIRROR=true
else
    # 提取纯数字，比如 0.850 -> 850
    MS=$(echo "$TIME_STR" | tr -d '.' | sed 's/^0*//')
    if [ -z "$MS" ]; then
        MS=999
    fi
    if [ "$MS" -gt 800 ]; then
        USE_MIRROR=true
    fi
fi

if [ "$USE_MIRROR" = true ]; then
    echo -e "\033[0;33mOfficial Rust server is slow (response time: ${TIME_STR}s). Enabling Rsproxy mirror for fast download...\033[0m"
    export RUSTUP_DIST_SERVER="https://rsproxy.cn"
    export RUSTUP_UPDATE_ROOT="https://rsproxy.cn/rustup"
fi

if ! command -v cargo &> /dev/null; then
    echo "Rust/Cargo not found. Installing via rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    # 将当前的 Cargo 路径写入会话环境
    source "$HOME/.cargo/env"
else
    echo "Rust environment detected: $(cargo --version)"
fi

# 3. 配置 Cargo 的 Crates 镜像以加速依赖包下载
if [ "$USE_MIRROR" = true ]; then
    echo "Setting up high-speed Cargo registry mirror (Rsproxy)..."
    mkdir -p "$HOME/.cargo"
    CARGO_CONFIG="$HOME/.cargo/config.toml"
    
    # 如果用户的 config.toml 里还没配过镜像，我们就写入
    if [ ! -f "$CARGO_CONFIG" ] || ! grep -q "rsproxy" "$CARGO_CONFIG"; then
        # 备份已有的配置
        if [ -f "$CARGO_CONFIG" ]; then
            cp "$CARGO_CONFIG" "${CARGO_CONFIG}.bak"
        fi
        
        cat << 'EOF' > "$CARGO_CONFIG"
[source.crates-io]
replace-with = 'rsproxy'

[source.rsproxy]
registry = "https://rsproxy.cn/crates.io-index"

[source.rsproxy-sparse]
registry = "sparse+https://rsproxy.cn/index/"

[registries.rsproxy]
index = "https://rsproxy.cn/crates.io-index"

[net]
git-fetch-with-cli = true
EOF
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
