#!/bin/bash
set -e

echo "=== Video Clipper Bootstrapper & Dependency Installer ==="

# 1. Check/Install Git
if ! command -v git &> /dev/null; then
    echo "Git is missing. Installing git..."
    if [[ "$OSTYPE" == "darwin"* ]]; then
        if ! command -v brew &> /dev/null; then
            echo "Homebrew is required to install Git and dependencies. Please install Homebrew: https://brew.sh/"
            exit 1
        fi
        brew install git
    else
        sudo apt update && sudo apt install -y git
    fi
fi

# 2. Check/Install ADB, FFmpeg, Node.js & Rust
if [[ "$OSTYPE" == "darwin"* ]]; then
    # macOS installation
    if ! command -v brew &> /dev/null; then
        echo "Homebrew is not installed. Please install it first: https://brew.sh/"
        exit 1
    fi
    
    if ! command -v adb &> /dev/null; then
        echo "Installing ADB..."
        brew install android-platform-tools
    else
        echo "ADB is already installed."
    fi
    
    if ! command -v ffmpeg &> /dev/null; then
        echo "Installing FFmpeg..."
        brew install ffmpeg
    else
        echo "FFmpeg is already installed."
    fi
    
    if ! command -v node &> /dev/null; then
        echo "Installing Node.js..."
        brew install node
    else
        echo "Node.js is already installed."
    fi

    if ! command -v rustc &> /dev/null; then
        echo "Installing Rust compiler..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        source "$HOME/.cargo/env"
    else
        echo "Rust is already installed."
    fi
else
    # Linux installation
    if ! command -v adb &> /dev/null; then
        echo "Installing ADB..."
        sudo apt update && sudo apt install -y android-tools-adb
    else
        echo "ADB is already installed."
    fi
    
    if ! command -v ffmpeg &> /dev/null; then
        echo "Installing FFmpeg..."
        sudo apt update && sudo apt install -y ffmpeg
    else
        echo "FFmpeg is already installed."
    fi
    
    if ! command -v node &> /dev/null; then
        echo "Installing Node.js & NPM..."
        curl -fsSL https://deb.nodesource.com/setup_lts.x | sudo -E bash -
        sudo apt install -y nodejs
    else
        echo "Node.js is already installed."
    fi

    # Install Tauri Linux system dependencies
    echo "Installing Tauri compilation dependencies (build-essential, GTK, WebKit, SSL)..."
    sudo apt install -y build-essential curl wget file libssl-dev libgtk-3-dev libwebkit2gtk-4.0-dev libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf

    if ! command -v rustc &> /dev/null; then
        echo "Installing Rust compiler..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        source "$HOME/.cargo/env"
    else
        echo "Rust is already installed."
    fi
fi

# 3. Clone Repository
echo "Cloning Video Clipper repository..."
if [ -d "video-clipper" ]; then
    echo "Folder 'video-clipper' already exists. Pulling latest updates..."
    cd video-clipper
    git pull
else
    git clone https://github.com/bassemsab/video-clipper.git
    cd video-clipper
fi

# 4. Install npm packages
echo "Installing application dependencies..."
npm install

# 5. Build and install to Applications folder on macOS
if [[ "$OSTYPE" == "darwin"* ]]; then
    # Make sure Rust compiler is in path
    export PATH="$HOME/.cargo/bin:$PATH"
    
    echo "Building production application bundle (this may take a couple of minutes on first run)..."
    npm run tauri build
    
    echo "Installing Video Clipper to /Applications..."
    # Remove old version if exists
    if [ -d "/Applications/Video Clipper.app" ]; then
        rm -rf "/Applications/Video Clipper.app"
    fi
    
    cp -R "src-tauri/target/release/bundle/macos/Video Clipper.app" /Applications/
    
    echo "===================================================="
    echo "🎉 Video Clipper has been successfully installed!"
    echo "You can now find 'Video Clipper' in your Applications folder/Launchpad."
    echo "===================================================="
else
    echo "===================================================="
    echo "🎉 Setup Completed Successfully!"
    echo "To run in development mode:"
    echo "  cd video-clipper && npm run tauri dev"
    echo "===================================================="
fi
