# Implementation Plan: iOS Simulator Support

## Goal
Extend the Video Clipper application to support auto-detecting and screen-recording iOS Simulators (in addition to Android devices).

---

## 1. Rust Backend Changes (`src-tauri/src/lib.rs`)

### 1.1 Device Struct Extension
Add a `platform` field to differentiate Android and iOS devices:
```rust
#[derive(Serialize, Deserialize, Debug, Clone)]
struct Device {
    name: String,
    state: String,
    display_name: String,
    platform: String, // "android" | "ios"
}
```

### 1.2 Booted Simulator Detection
Create a helper to query booted iOS Simulators using macOS's built-in `xcrun simctl` command:
```rust
fn get_ios_simulators() -> Vec<Device> {
    let mut sims = vec![];
    // Run: xcrun simctl list devices -j
    // Parse the JSON output to find devices with state == "Booted"
    // Return them as Device structs with platform == "ios"
    sims
}
```
*Note: We can merge simulator items into the device list returned by `get_adb_status` (and rename the API command to something like `get_devices_status` or keep it unified).*

### 1.3 Simulator Screen Recording
Add commands to record booted Simulators:
* **Start Recording**: Runs `xcrun simctl io <UDID> recordVideo <output_path>`.
* **Stop Recording**: Sends `SIGINT` (Ctrl+C) to the recording process to finalize the `.mp4` file.

---

## 2. Frontend Changes (`src/main.js` & `src/index.html`)

### 2.1 UI badges
* Render iOS Simulators inside the device modal list with a distinct **`iOS Simulator`** badge.
* Hide pairing options for iOS simulators since they are already active/local.

### 2.2 Triggering Logic
* When starting/stopping recording, verify the `platform` of the active device.
* If `activeDevice.platform == "ios"`, call the iOS recording Tauri backend commands instead of the Android ADB ones.
* Video loading and frame clipping remain exactly the same since both output standard `.mp4` files!

---

## 3. Verification Plan
* Run an iOS Simulator in Xcode/Simulator.app.
* Open the Video Clipper settings modal. Verify the simulator is listed as `Booted` with the `iOS Simulator` badge.
* Select it, click record, trigger simulator interactions, stop, and verify frames extract correctly!
