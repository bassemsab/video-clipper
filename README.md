# Interactive Video Clipper & ADB Screen Recorder

A premium, cross-platform desktop application built with Tauri, Rust, and Vanilla JavaScript/CSS designed for developers, testers, and AI engineers. 

### 🎯 The Core Goal
The primary objective of this tool is to **make it easy to give Agentic AI coding assistants visual feedback on the results of their work**, specifically when it comes to frontend user interface changes, micro-animations, and layout optimizations.

When pair-programming with AI agents, they cannot directly "see" the visual output on a device. By recording your device screen wirelessly, clipping timeline regions, and generating structured frame grids directly into your clipboard, you can instantly paste the visual evidence back to the AI. This creates a seamless visual feedback loop so the AI can analyze rendering issues, verify design alignment, and iterate on fixes with high precision.

---

## ⚡ Quick Start (One-liner Installation)

You can install all system dependencies (ADB, FFmpeg, Node.js, and Rust), clone the repository, and set up everything automatically in one go:

### 🍏 macOS / 🐧 Linux
```bash
curl -fsSL https://raw.githubusercontent.com/bassemsab/video-clipper/main/install.sh | bash
```

### 🪟 Windows
Run the following inside an administrator PowerShell terminal:
```powershell
Set-ExecutionPolicy Bypass -Scope Process -Force; [System.Net.ServicePointManager]::SecurityProtocol = [System.Net.SecurityProtocolType]::Tls12; iex ((New-Object System.Net.WebClient).DownloadString('https://raw.githubusercontent.com/bassemsab/video-clipper/main/install.ps1'))
```

---

## 🚀 Key Features

* **Wireless ADB Screen Recording**: Toggle developer options on your Android phone, pair once, and connect wirelessly. The app remembers paired device IPs and supports instant port adjustments.
* **Persistent Pairing History**: Your paired devices list is stored in a clean local database. One-click reconnect keeps you in flow.
* **Custom FPS Frame Extraction**: Key in any frame rate from **`1` to `320` FPS** using a custom numeric input. The target paths are updated dynamically.
* **Save to Disk**: Save your clipped segment frame-by-frame as raw PNG images inside organized folders.
* **📋 Copy to Clipboard**: 
  - **Copy Current Frame**: Copies the paused video frame instantly without writing files to disk.
  - **Copy Frames Grid**: Seek and sample frames across the selected timeline region at your custom FPS. It combines all frames into a single high-quality grid image, overlayed with frame index and millisecond offset labels (e.g. `#12 (+0.733s)`). Perfect for pasting directly into Gemini, ChatGPT, or Claude for rendering analysis.
* **Keyboard Shortcuts**: Fine-tuned control over skipping, stepping, and playing frames with sub-millisecond precision.

---

## 📦 System Dependencies

To run or build the application, ensure the following binary packages are installed on your host system:

### 1. FFmpeg (Required for frame extraction operations)
* **macOS**: `brew install ffmpeg`
* **Windows**: `choco install ffmpeg` or download from official sources and add to environment path.
* **Linux**: `sudo apt update && sudo apt install ffmpeg`

### 2. ADB (Android Debug Bridge, required for screen recording)
* **macOS**: `brew install android-platform-tools`
* **Windows**: `choco install adb`
* **Linux**: `sudo apt update && sudo apt install android-tools-adb`

---

## 🛠️ Development Setup

Ensure you have Rust, Node.js, and npm installed on your machine.

1. **Clone the repository**:
   ```bash
   git clone https://github.com/bassemsab/video-clipper.git
   cd video-clipper
   ```

2. **Install dependencies**:
   ```bash
   npm install
   ```

3. **Run in development mode**:
   ```bash
   npm run tauri dev
   ```

---

## 🏗️ Building for Production (Mac, Windows, Linux)

Tauri compiles native platform bundles locally. Run the build command on your target platform to generate distribution packages:

### macOS (creates `.app` bundle & `.dmg` installer)
```bash
npm run tauri build
```

### Windows (creates `.msi` installer)
```bash
npm run tauri build
```

### Linux (creates `.deb` package & `.AppImage`)
```bash
npm run tauri build
```

---

## ⌨️ Keyboard Shortcuts

| Shortcut | Action |
| --- | --- |
| `Space` | Play / Pause video playback |
| `←` / `→` | Skip back / forward 1 second |
| `,` / `.` | Step back / forward 1 exact frame (1/60s) |

---

## 📝 License

Open-source under the MIT License. Created by [bassemsab](https://github.com/bassemsab).
