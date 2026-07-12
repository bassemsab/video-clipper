#!/bin/bash
set -e

echo "=== Video Clipper Bootstrapper & Dependency Installer ==="

# 1. Check/Install Git
if ! command -v git &> /dev/null; then
    echo "Git is missing. Installing git..."
    if [ "$(uname)" = "Darwin" ]; then
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
if [ "$(uname)" = "Darwin" ]; then
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
if [ "$(uname)" = "Darwin" ]; then
    # Make sure Rust compiler is in path
    export PATH="$HOME/.cargo/bin:$PATH"
    
    # Check if app is in use before building to save time
    if pgrep -f "/Applications/Video Clipper.app" >/dev/null; then
        echo "⚠️  Error: Video Clipper is currently running from /Applications."
        echo "    Please quit the application and run this installer again."
        exit 1
    fi

    echo "Building production application bundle (this may take a couple of minutes on first run)..."
    npm run tauri build
    
    install_with_spinner() {
        local build_path="src-tauri/target/release/bundle/macos/Video Clipper.app"
        local dest_path="/Applications/Video Clipper.app"
        
        # Double check if running (in case they opened it during build)
        if pgrep -f "$dest_path" >/dev/null; then
            echo "⚠️  Error: Video Clipper is currently running from /Applications."
            echo "    Please quit the application and run this installer again."
            exit 1
        fi

        local spin='-\|/'
        local i=0
        
        # 1. Clean up old version if it exists
        if [ -d "$dest_path" ]; then
            echo -n "Removing old version of Video Clipper from /Applications... "
            rm -rf "$dest_path" &
            local rm_pid=$!
            while kill -0 $rm_pid 2>/dev/null; do
                i=$(( (i+1) % 4 ))
                printf "\rRemoving old version of Video Clipper from /Applications... %s" "${spin:$i:1}"
                sleep 0.1
            done
            wait $rm_pid
            printf "\rRemoving old version of Video Clipper from /Applications... Done!\n"
        fi
        
        # 2. Move new build to /Applications/
        if [ -d "$build_path" ]; then
            echo -n "Installing Video Clipper to /Applications... "
            mv "$build_path" "/Applications/" &
            local mv_pid=$!
            while kill -0 $mv_pid 2>/dev/null; do
                i=$(( (i+1) % 4 ))
                printf "\rInstalling Video Clipper to /Applications... %s" "${spin:$i:1}"
                sleep 0.1
            done
            wait $mv_pid
            printf "\rInstalling Video Clipper to /Applications... Done!\n"
        else
            echo "❌ Error: Compiled bundle not found at $build_path"
            exit 1
        fi
    }

    install_with_spinner

    echo "===================================================="
    echo "🎉 Video Clipper build & installation completed successfully!"
    echo "You can find your installation at:"
    echo "  App Location:  /Applications/Video Clipper.app"
    echo "  DMG Backup:    src-tauri/target/release/bundle/dmg/"
    echo "===================================================="
else
    echo "===================================================="
    echo "🎉 Setup Completed Successfully!"
    echo "To run in development mode:"
    echo "  cd video-clipper && npm run tauri dev"
    echo "===================================================="
fi
