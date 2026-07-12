# Video Clipper Agent Instructions & Constraints

## 1. Environment & Paths
* **Binary Location**: GUI apps on macOS do not inherit environment shell paths (like `/opt/homebrew/bin`). Always search standard installation paths (`/opt/homebrew/bin`, `/usr/local/bin`, `/usr/bin`, `/bin`) using the `find_in_path` helper function when invoking binary tools like `adb`, `ffmpeg`, or Xcode-related tools.
* **No hardcoded paths**: Avoid hardcoding absolute system binary paths. Use the dynamic path locators in `src-tauri/src/lib.rs`.

## 2. Clipboard Handling
* **Native Clipboard Only**: Do not use `navigator.clipboard.write()` for image files in the frontend. WebView sandboxes block asynchronous clipboard writes (e.g. after grid rendering promises) due to expired user gestures. Always invoke the custom native command `write_image_to_clipboard` with base64 image data.

## 3. ADB & Devices List
* **Internal ADB Name vs Display Name**: When modifying device names, do not alter the `name` field in the Rust `Device` struct (it must remain the raw ADB selector string like `adb-0A091FDD40076S-FUhOgx...` for targeting commands). Always use the `display_name` field for human-readable labels in the list.
* **Deduplication**: Keep the model-based filtering in `get_adb_status` active so that cryptic auto-connected mDNS channels are hidden if the device is already manually connected via IP.

## 4. UI/UX Rules
* **No Sidebar Status Text**: Do not place long status or error message tags directly in the sidebar panel. Use the floating `showToast(message, type)` notification system to keep the layout spacious and prevent label overlapping.

## 5. Build Procedures
* **No Automatic Production Builds**: Never run `npx tauri build` or generate production installer bundles (`.dmg`, `.app`) automatically. Only compile release builds when the user explicitly requests one. For development iterations, rely on dev mode (`npm run tauri dev`) or simple checks (`cargo check`).

