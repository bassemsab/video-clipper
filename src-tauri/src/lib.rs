use std::process::{Command, Child, Stdio};
use std::sync::Mutex;
use std::path::{Path, PathBuf};
use std::time::Duration;
use serde::{Serialize, Deserialize};
use tauri::{State, AppHandle};

#[derive(Default)]
struct AppState {
    recording_process: Mutex<Option<Child>>,
    video_path: Mutex<Option<String>>,
    recording_platform: Mutex<Option<String>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Device {
    name: String,
    state: String,
    display_name: String,
    platform: String, // "android" | "ios"
}

#[derive(Deserialize, Debug)]
struct SimctlDevice {
    state: String,
    name: String,
    udid: String,
    #[serde(rename = "isAvailable")]
    is_available: Option<bool>,
}

#[derive(Deserialize, Debug)]
struct SimctlOutput {
    devices: std::collections::HashMap<String, Vec<SimctlDevice>>,
}

fn get_ios_simulators() -> Vec<Device> {
    let xcrun_path = match find_in_path("xcrun") {
        Some(path) => path,
        None => return vec![],
    };

    let output = match Command::new(&xcrun_path)
        .arg("simctl")
        .arg("list")
        .arg("devices")
        .arg("-j")
        .output()
    {
        Ok(out) => out,
        Err(_) => return vec![],
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let sim_output: SimctlOutput = match serde_json::from_str(&stdout) {
        Ok(parsed) => parsed,
        Err(_) => return vec![],
    };

    let mut devices = vec![];
    for (_runtime, sim_devices) in sim_output.devices {
        for sim in sim_devices {
            if sim.state == "Booted" && sim.is_available.unwrap_or(true) {
                devices.push(Device {
                    name: sim.udid,
                    state: "device".to_string(),
                    display_name: sim.name,
                    platform: "ios".to_string(),
                });
            }
        }
    }
    devices
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct AdbStatus {
    installed: bool,
    devices: Vec<Device>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ApiResponse<T> {
    success: bool,
    message: String,
    data: Option<T>,
    error: Option<String>,
    needs_manual_connect: Option<bool>,
}

fn find_in_path(cmd: &str) -> Option<PathBuf> {
    // 1. Try environment PATH
    if let Ok(path_var) = std::env::var("PATH") {
        for path in std::env::split_paths(&path_var) {
            let exe_path = path.join(cmd);
            if exe_path.is_file() {
                return Some(exe_path);
            }
        }
    }
    // 2. Try common system folders (e.g. on macOS GUI app where PATH is limited)
    let fallbacks = [
        "/opt/homebrew/bin",
        "/usr/local/bin",
        "/usr/bin",
        "/bin",
    ];
    for path in &fallbacks {
        let exe_path = Path::new(path).join(cmd);
        #[cfg(target_os = "windows")]
        let exe_path = Path::new(path).join(format!("{}.exe", cmd));
        
        if exe_path.is_file() {
            return Some(exe_path);
        }
    }
    None
}

fn find_adb() -> Option<PathBuf> {
    // 1. Try PATH and fallback system folders
    if let Some(path) = find_in_path("adb") {
        return Some(path);
    }
    // 2. Try default macOS Android SDK path
    if let Ok(home) = std::env::var("HOME") {
        let sdk_path = Path::new(&home).join("Library/Android/sdk/platform-tools/adb");
        if sdk_path.exists() {
            return Some(sdk_path);
        }
    }
    None
}

fn find_ffmpeg() -> Option<PathBuf> {
    find_in_path("ffmpeg")
}



#[tauri::command]
fn get_adb_status() -> AdbStatus {
    let mut devices = vec![];
    let mut adb_installed = false;

    if let Some(adb_path) = find_adb() {
        adb_installed = true;
        if let Ok(output) = Command::new(&adb_path).arg("devices").arg("-l").output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let mut connected_names = std::collections::HashSet::new();
            let mut connected_models = std::collections::HashSet::new();

            // 1. First Pass: Gather model names of connected manual IP connections
            let lines: Vec<&str> = stdout.lines().collect();
            if lines.len() > 1 {
                for line in &lines[1..] {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        let name = parts[0].trim().to_string();
                        let state = parts[1].trim().to_string();
                        
                        if !name.is_empty() && state == "device" && name.contains(':') {
                            // Extract model value (e.g. model:Pixel_5 -> Pixel_5)
                            for part in &parts[2..] {
                                if part.starts_with("model:") {
                                    let model_name = part.split(':').nth(1).unwrap_or("").to_string();
                                    if !model_name.is_empty() {
                                        connected_models.insert(model_name);
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // 2. Second Pass: Process devices and skip duplicate mDNS auto-connections
            if lines.len() > 1 {
                for line in &lines[1..] {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        let name = parts[0].trim().to_string();
                        let state = parts[1].trim().to_string();
                        if !name.is_empty() {
                            let mut display_name = name.clone();
                            let mut is_mdns = false;
                            let mut model_name = String::new();

                            // Parse model name for this device
                            for part in &parts[2..] {
                                if part.starts_with("model:") {
                                    model_name = part.split(':').nth(1).unwrap_or("").to_string();
                                }
                            }

                            if name.contains("_adb-tls-connect") {
                                is_mdns = true;
                                let base_name = name.split('.').next().unwrap_or(&name).trim();
                                let clean_base = base_name.split_whitespace().next().unwrap_or(base_name);
                                let without_prefix = if clean_base.starts_with("adb-") {
                                    &clean_base[4..]
                                } else {
                                    clean_base
                                };
                                let serial = without_prefix.split('-').next().unwrap_or(without_prefix);
                                display_name = format!("{} (Wireless)", serial);
                            }

                            // Skip duplicate mDNS connection if we already have a manual IP connection to the same device model
                            if is_mdns && !model_name.is_empty() && connected_models.contains(&model_name) {
                                continue;
                            }

                            if !connected_names.contains(&name) {
                                devices.push(Device { name: name.clone(), state, display_name, platform: "android".to_string() });
                                connected_names.insert(name);
                            }
                        }
                    }
                }
            }

            // Try to find paired but disconnected devices via mDNS
            if let Ok(mdns_out) = Command::new(&adb_path).arg("mdns").arg("services").output() {
                let mdns_stdout = String::from_utf8_lossy(&mdns_out.stdout);
                for line in mdns_stdout.lines() {
                    if line.contains("_adb-tls-connect") {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        for part in parts {
                            if part.contains(':') && !connected_names.contains(part) {
                                let name_str = part.to_string();
                                devices.push(Device {
                                    name: name_str.clone(),
                                    state: "available (paired)".to_string(),
                                    display_name: name_str,
                                    platform: "android".to_string(),
                                });
                                connected_names.insert(part.to_string());
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    // Append iOS simulators
    let mut ios_sims = get_ios_simulators();
    devices.append(&mut ios_sims);

    let has_simctl = cfg!(target_os = "macos") && find_in_path("xcrun").is_some();
    let installed = adb_installed || has_simctl;

    AdbStatus { installed, devices }
}

#[tauri::command]
fn adb_pair(ip_port: String, code: String) -> ApiResponse<String> {
    let adb_path = match find_adb() {
        Some(path) => path,
        None => return ApiResponse { success: false, message: "".to_string(), data: None, error: Some("ADB not installed".to_string()), needs_manual_connect: None }
    };

    let output = match Command::new(&adb_path).arg("pair").arg(&ip_port).arg(&code).output() {
        Ok(out) => out,
        Err(e) => return ApiResponse { success: false, message: "".to_string(), data: None, error: Some(e.to_string()), needs_manual_connect: None }
    };

    let res_str = format!("{}{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
    
    if res_str.contains("Successfully paired") || res_str.to_lowercase().contains("successfully") {
        // Try mDNS auto-connect
        std::thread::sleep(Duration::from_secs(1));
        let base_ip = ip_port.split(':').next().unwrap_or("").to_string();
        let mut auto_connected = false;

        if let Ok(mdns_out) = Command::new(&adb_path).arg("mdns").arg("services").output() {
            let mdns_stdout = String::from_utf8_lossy(&mdns_out.stdout);
            for line in mdns_stdout.lines() {
                if line.contains(&base_ip) && line.contains("_adb-tls-connect") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    for part in parts {
                        if part.contains(&base_ip) && part.contains(':') {
                            if let Ok(conn_out) = Command::new(&adb_path).arg("connect").arg(part).output() {
                                let conn_str = String::from_utf8_lossy(&conn_out.stdout).to_lowercase();
                                if conn_str.contains("connected") {
                                    auto_connected = true;
                                    break;
                                }
                            }
                        }
                    }
                    if auto_connected { break; }
                }
            }
        }

        if auto_connected {
            ApiResponse {
                success: true,
                message: "Successfully paired and auto-connected!".to_string(),
                data: Some("connected".to_string()),
                error: None,
                needs_manual_connect: Some(false),
            }
        } else {
            ApiResponse {
                success: true,
                message: "Paired successfully, but auto-connect failed. Please enter the main port.".to_string(),
                data: Some("paired".to_string()),
                error: None,
                needs_manual_connect: Some(true),
            }
        }
    } else {
        ApiResponse {
            success: false,
            message: "".to_string(),
            data: None,
            error: Some(res_str),
            needs_manual_connect: None,
        }
    }
}

#[tauri::command]
fn adb_connect(ip_port: String) -> ApiResponse<String> {
    let adb_path = match find_adb() {
        Some(path) => path,
        None => return ApiResponse { success: false, message: "".to_string(), data: None, error: Some("ADB not installed".to_string()), needs_manual_connect: None }
    };

    let output = match Command::new(&adb_path).arg("connect").arg(&ip_port).output() {
        Ok(out) => out,
        Err(e) => return ApiResponse { success: false, message: "".to_string(), data: None, error: Some(e.to_string()), needs_manual_connect: None }
    };

    let res_str = format!("{}{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
    if res_str.to_lowercase().contains("connected") || res_str.to_lowercase().contains("already connected") {
        ApiResponse {
            success: true,
            message: res_str,
            data: Some("connected".to_string()),
            error: None,
            needs_manual_connect: None,
        }
    } else {
        ApiResponse {
            success: false,
            message: "".to_string(),
            data: None,
            error: Some(res_str),
            needs_manual_connect: None,
        }
    }
}

#[tauri::command]
fn adb_disconnect(ip_port: String) -> ApiResponse<String> {
    let adb_path = match find_adb() {
        Some(path) => path,
        None => return ApiResponse { success: false, message: "".to_string(), data: None, error: Some("ADB not installed".to_string()), needs_manual_connect: None }
    };

    let _output = match Command::new(&adb_path).arg("disconnect").arg(&ip_port).output() {
        Ok(out) => out,
        Err(e) => return ApiResponse { success: false, message: "".to_string(), data: None, error: Some(e.to_string()), needs_manual_connect: None }
    };

    ApiResponse {
        success: true,
        message: "Disconnected".to_string(),
        data: None,
        error: None,
        needs_manual_connect: None,
    }
}

#[tauri::command]
fn load_video(path: String, state: State<'_, AppState>) -> ApiResponse<String> {
    if !Path::new(&path).exists() {
        return ApiResponse { success: false, message: "".to_string(), data: None, error: Some("File does not exist".to_string()), needs_manual_connect: None };
    }
    let mut video_path = state.video_path.lock().unwrap();
    *video_path = Some(path.clone());
    ApiResponse {
        success: true,
        message: format!("Video loaded: {}", path),
        data: Some(path),
        error: None,
        needs_manual_connect: None,
    }
}

#[tauri::command]
fn start_recording(device_id: Option<String>, state: State<'_, AppState>) -> ApiResponse<String> {
    // Check if device_id is an iOS simulator
    let is_ios = if let Some(ref id) = device_id {
        get_ios_simulators().iter().any(|d| &d.name == id)
    } else {
        false
    };

    if is_ios {
        let xcrun_path = match find_in_path("xcrun") {
            Some(path) => path,
            None => return ApiResponse { success: false, message: "".to_string(), data: None, error: Some("xcrun not found".to_string()), needs_manual_connect: None }
        };

        let mut proc_lock = state.recording_process.lock().unwrap();
        if let Some(ref mut child) = *proc_lock {
            if child.try_wait().map(|s| s.is_none()).unwrap_or(false) {
                return ApiResponse { success: false, message: "".to_string(), data: None, error: Some("Already recording".to_string()), needs_manual_connect: None };
            } else {
                *proc_lock = None;
            }
        }

        let output_dir = PathBuf::from("/Users/bassem/Documents/projects/video-clipper/recordings");
        let _ = std::fs::create_dir_all(&output_dir);
        let local_path = output_dir.join("ios_record.mp4");
        
        if local_path.exists() {
            let _ = std::fs::remove_file(&local_path);
        }

        let mut rec_cmd = Command::new(&xcrun_path);
        rec_cmd.arg("simctl")
               .arg("io")
               .arg(device_id.as_ref().unwrap())
               .arg("recordVideo")
               .arg("--force")
               .arg(&local_path);

        rec_cmd.stderr(Stdio::piped());

        match rec_cmd.spawn() {
            Ok(child) => {
                *proc_lock = Some(child);
                let mut plat_lock = state.recording_platform.lock().unwrap();
                *plat_lock = Some("ios".to_string());

                ApiResponse {
                    success: true,
                    message: "iOS Simulator recording started".to_string(),
                    data: Some("recording".to_string()),
                    error: None,
                    needs_manual_connect: None,
                }
            }
            Err(e) => {
                ApiResponse {
                    success: false,
                    message: "".to_string(),
                    data: None,
                    error: Some(format!("Failed to start simctl recordVideo: {}", e)),
                    needs_manual_connect: None,
                }
            }
        }
    } else {
        // Android recording workflow
        let adb_path = match find_adb() {
            Some(path) => path,
            None => return ApiResponse { success: false, message: "".to_string(), data: None, error: Some("ADB not installed".to_string()), needs_manual_connect: None }
        };

        let mut proc_lock = state.recording_process.lock().unwrap();
        if let Some(ref mut child) = *proc_lock {
            if child.try_wait().map(|s| s.is_none()).unwrap_or(false) {
                return ApiResponse { success: false, message: "".to_string(), data: None, error: Some("Already recording".to_string()), needs_manual_connect: None };
            } else {
                *proc_lock = None;
            }
        }

        let dev_id_ref = device_id.as_deref();
        if let Some(id) = dev_id_ref {
            if id.contains(':') {
                // Auto connect before recording if Wi-Fi
                let _ = Command::new(&adb_path).arg("connect").arg(id).output();
            }
        }

        // Verify a device is indeed connected
        let mut check_cmd = Command::new(&adb_path);
        if let Some(id) = dev_id_ref {
            check_cmd.arg("-s").arg(id);
        }
        check_cmd.arg("shell").arg("getprop").arg("sys.boot_completed");
        
        match check_cmd.output() {
            Ok(out) if out.status.success() => {},
            _ => return ApiResponse { success: false, message: "".to_string(), data: None, error: Some("Selected ADB device is not connected or not responding.".to_string()), needs_manual_connect: None }
        }

        // Wake up the device if screen is off
        let mut wake_cmd = Command::new(&adb_path);
        if let Some(id) = dev_id_ref {
            wake_cmd.arg("-s").arg(id);
        }
        let _ = wake_cmd.arg("shell").arg("input").arg("keyevent").arg("224").output(); // KEYCODE_WAKE

        let mut rec_cmd = Command::new(&adb_path);
        if let Some(id) = dev_id_ref {
            rec_cmd.arg("-s").arg(id);
        }
        rec_cmd.arg("shell").arg("screenrecord").arg("/sdcard/clip_record.mp4");
        rec_cmd.stderr(Stdio::piped());

        match rec_cmd.spawn() {
            Ok(mut child) => {
                // Wait 400ms to see if screenrecord fails instantly (e.g. locked display state)
                std::thread::sleep(Duration::from_millis(400));
                if let Ok(Some(_status)) = child.try_wait() {
                    let mut err_msg = "screenrecord exited prematurely. Please make sure your phone's screen is ON and UNLOCKED.".to_string();
                    if let Some(mut stderr) = child.stderr.take() {
                        use std::io::Read;
                        let mut buf = String::new();
                        if stderr.read_to_string(&mut buf).is_ok() && !buf.trim().is_empty() {
                            if buf.contains("INVALID_LAYER_STACK") {
                                err_msg = "Device screen is locked or turned off. Please unlock the screen and try again.".to_string();
                            } else {
                                err_msg = format!("screenrecord error: {}", buf.trim());
                            }
                        }
                    }
                    return ApiResponse {
                        success: false,
                        message: "".to_string(),
                        data: None,
                        error: Some(err_msg),
                        needs_manual_connect: None,
                    };
                }

                *proc_lock = Some(child);
                let mut plat_lock = state.recording_platform.lock().unwrap();
                *plat_lock = Some("android".to_string());

                ApiResponse {
                    success: true,
                    message: "Recording started".to_string(),
                    data: Some("recording".to_string()),
                    error: None,
                    needs_manual_connect: None,
                }
            },
            Err(e) => {
                ApiResponse {
                    success: false,
                    message: "".to_string(),
                    data: None,
                    error: Some(format!("Failed to start screenrecord: {}", e)),
                    needs_manual_connect: None,
                }
            }
        }
    }
}

#[tauri::command]
fn stop_recording(device_id: Option<String>, state: State<'_, AppState>, _app: AppHandle) -> ApiResponse<String> {
    let platform = {
        let plat_lock = state.recording_platform.lock().unwrap();
        plat_lock.clone().unwrap_or_else(|| "android".to_string())
    };

    let mut proc_lock = state.recording_process.lock().unwrap();
    let mut child = match proc_lock.take() {
        Some(c) => c,
        None => return ApiResponse { success: false, message: "".to_string(), data: None, error: Some("Not recording".to_string()), needs_manual_connect: None }
    };

    if platform == "ios" {
        // Send SIGINT (2) on macOS to let simctl stop and finalize the video correctly
        #[cfg(target_os = "macos")]
        {
            let pid = child.id();
            let _ = Command::new("kill").arg("-2").arg(pid.to_string()).output();
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = child.kill();
        }

        let _ = child.wait();
        std::thread::sleep(Duration::from_secs(1));

        let output_dir = PathBuf::from("/Users/bassem/Documents/projects/video-clipper/recordings");
        let local_path = output_dir.join("ios_record.mp4");
        let local_path_str = local_path.to_string_lossy().to_string();

        if local_path.exists() {
            let mut vpath_lock = state.video_path.lock().unwrap();
            *vpath_lock = Some(local_path_str.clone());

            ApiResponse {
                success: true,
                message: "iOS recording saved".to_string(),
                data: Some(local_path_str),
                error: None,
                needs_manual_connect: None,
            }
        } else {
            ApiResponse {
                success: false,
                message: "".to_string(),
                data: None,
                error: Some("iOS recording file not found after stopping".to_string()),
                needs_manual_connect: None,
            }
        }
    } else {
        // Android workflow
        let adb_path = match find_adb() {
            Some(path) => path,
            None => return ApiResponse { success: false, message: "".to_string(), data: None, error: Some("ADB not installed".to_string()), needs_manual_connect: None }
        };

        let _ = child.kill();
        let _ = child.wait();

        std::thread::sleep(Duration::from_secs(1));

        // Resolve local path to pull the video
        let output_dir = PathBuf::from("/Users/bassem/Documents/projects/video-clipper/recordings");
        let _ = std::fs::create_dir_all(&output_dir);
        let local_path = output_dir.join("android_record.mp4");
        let local_path_str = local_path.to_string_lossy().to_string();

        let dev_id_ref = device_id.as_deref();

        // Pull video
        let mut pull_cmd = Command::new(&adb_path);
        if let Some(id) = dev_id_ref {
            pull_cmd.arg("-s").arg(id);
        }
        pull_cmd.arg("pull").arg("/sdcard/clip_record.mp4").arg(&local_path);

        if let Ok(out) = pull_cmd.output() {
            if out.status.success() {
                // Delete remote video
                let mut rm_cmd = Command::new(&adb_path);
                if let Some(id) = dev_id_ref {
                    rm_cmd.arg("-s").arg(id);
                }
                let _ = rm_cmd.arg("shell").arg("rm").arg("/sdcard/clip_record.mp4").output();

                let mut vpath_lock = state.video_path.lock().unwrap();
                *vpath_lock = Some(local_path_str.clone());

                return ApiResponse {
                    success: true,
                    message: "Android recording saved".to_string(),
                    data: Some(local_path_str),
                    error: None,
                    needs_manual_connect: None,
                };
            }
        }

        ApiResponse {
            success: false,
            message: "".to_string(),
            data: None,
            error: Some("Failed to pull recording from Android".to_string()),
            needs_manual_connect: None,
        }
    }
}

#[tauri::command]
fn extract_frames(start: f64, end: f64, fps: String, state: State<'_, AppState>) -> ApiResponse<String> {
    let video_path = {
        let lock = state.video_path.lock().unwrap();
        match *lock {
            Some(ref p) => p.clone(),
            None => return ApiResponse { success: false, message: "".to_string(), data: None, error: Some("No video loaded".to_string()), needs_manual_connect: None }
        }
    };

    let video_path_buf = PathBuf::from(&video_path);
    let video_dir = video_path_buf.parent().unwrap_or(Path::new("."));
    let video_name = video_path_buf.file_stem().unwrap_or(std::ffi::OsStr::new("video")).to_string_lossy();
    let folder_name = format!("{}_frames_{}fps", video_name, fps);
    let output_dir = video_dir.join(folder_name);
    let _ = std::fs::create_dir_all(&output_dir);

    let output_pattern = output_dir.join("%04d.png");
    let duration = end - start;

    let ffmpeg_path = match find_ffmpeg() {
        Some(path) => path,
        None => return ApiResponse {
            success: false,
            message: "".to_string(),
            data: None,
            error: Some("ffmpeg binary not found. Please install ffmpeg and make sure it is in your system path.".to_string()),
            needs_manual_connect: None,
        }
    };

    let output = Command::new(&ffmpeg_path)
        .arg("-y")
        .arg("-ss").arg(start.to_string())
        .arg("-i").arg(&video_path)
        .arg("-t").arg(duration.to_string())
        .arg("-r").arg(&fps)
        .arg(&output_pattern)
        .output();

    match output {
        Ok(out) if out.status.success() => {
            ApiResponse {
                success: true,
                message: format!("Successfully extracted frames to {}", output_dir.to_string_lossy()),
                data: Some(output_dir.to_string_lossy().to_string()),
                error: None,
                needs_manual_connect: None,
            }
        },
        Ok(out) => {
            let err = String::from_utf8_lossy(&out.stderr);
            ApiResponse {
                success: false,
                message: "".to_string(),
                data: None,
                error: Some(format!("ffmpeg failed: {}", err)),
                needs_manual_connect: None,
            }
        },
        Err(e) => {
            ApiResponse {
                success: false,
                message: "".to_string(),
                data: None,
                error: Some(format!("Failed to execute ffmpeg: {}", e)),
                needs_manual_connect: None,
            }
        }
    }
}

#[tauri::command]
fn extract_frame(time: f64, state: State<'_, AppState>) -> ApiResponse<String> {
    let video_path = {
        let lock = state.video_path.lock().unwrap();
        match *lock {
            Some(ref p) => p.clone(),
            None => return ApiResponse { success: false, message: "".to_string(), data: None, error: Some("No video loaded".to_string()), needs_manual_connect: None }
        }
    };

    let video_path_buf = PathBuf::from(&video_path);
    let video_dir = video_path_buf.parent().unwrap_or(Path::new("."));
    let video_name = video_path_buf.file_stem().unwrap_or(std::ffi::OsStr::new("video")).to_string_lossy();
    let folder_name = format!("{}_frames", video_name);
    let output_dir = video_dir.join(folder_name);
    let _ = std::fs::create_dir_all(&output_dir);

    let filename = format!("frame_{:.3}.png", time).replace('.', "_");
    let output_path = output_dir.join(filename);

    let ffmpeg_path = match find_ffmpeg() {
        Some(path) => path,
        None => return ApiResponse {
            success: false,
            message: "".to_string(),
            data: None,
            error: Some("ffmpeg binary not found. Please install ffmpeg and make sure it is in your system path.".to_string()),
            needs_manual_connect: None,
        }
    };

    let output = Command::new(&ffmpeg_path)
        .arg("-y")
        .arg("-ss").arg(time.to_string())
        .arg("-i").arg(&video_path)
        .arg("-frames:v").arg("1")
        .arg(&output_path)
        .output();

    match output {
        Ok(out) if out.status.success() => {
            ApiResponse {
                success: true,
                message: format!("Successfully extracted single frame to {}", output_path.to_string_lossy()),
                data: Some(output_path.to_string_lossy().to_string()),
                error: None,
                needs_manual_connect: None,
            }
        },
        Ok(out) => {
            let err = String::from_utf8_lossy(&out.stderr);
            ApiResponse {
                success: false,
                message: "".to_string(),
                data: None,
                error: Some(format!("ffmpeg failed: {}", err)),
                needs_manual_connect: None,
            }
        },
        Err(e) => {
            ApiResponse {
                success: false,
                message: "".to_string(),
                data: None,
                error: Some(format!("Failed to execute ffmpeg: {}", e)),
                needs_manual_connect: None,
            }
        }
    }
}

#[tauri::command]
fn save_image_to_disk(base64_data: String, video_path: String, is_grid: bool) -> ApiResponse<String> {
    // 1. Decode base64 image data to bytes
    let clean_base64 = base64_data
        .replace("data:image/png;base64,", "")
        .replace("data:image/jpeg;base64,", "")
        .trim()
        .to_string();

    let bytes = match base64::Engine::decode(&base64::prelude::BASE64_STANDARD, &clean_base64) {
        Ok(b) => b,
        Err(e) => return ApiResponse { success: false, message: "".to_string(), data: None, error: Some(e.to_string()), needs_manual_connect: None }
    };

    // 2. Resolve output directory and filename based on original video path
    let video_path_buf = PathBuf::from(&video_path);
    let video_dir = video_path_buf.parent().unwrap_or(Path::new("."));
    let video_name = video_path_buf.file_stem().unwrap_or(std::ffi::OsStr::new("video")).to_string_lossy();
    
    let filename = if is_grid {
        format!("{}_grid.png", video_name)
    } else {
        format!("{}_frame.png", video_name)
    };
    
    let output_path = video_dir.join(filename);

    if let Err(e) = std::fs::write(&output_path, bytes) {
        return ApiResponse { success: false, message: "".to_string(), data: None, error: Some(e.to_string()), needs_manual_connect: None };
    }

    ApiResponse {
        success: true,
        message: format!("Grid image saved: {}", output_path.file_name().unwrap_or(std::ffi::OsStr::new("")).to_string_lossy()),
        data: Some(output_path.to_string_lossy().to_string()),
        error: None,
        needs_manual_connect: None,
    }
}

#[tauri::command]
fn write_image_to_clipboard(base64_data: String) -> ApiResponse<String> {
    // 1. Decode base64 image data to bytes
    let clean_base64 = base64_data
        .replace("data:image/png;base64,", "")
        .replace("data:image/jpeg;base64,", "")
        .trim()
        .to_string();

    let bytes = match base64::Engine::decode(&base64::prelude::BASE64_STANDARD, &clean_base64) {
        Ok(b) => b,
        Err(e) => return ApiResponse { success: false, message: "".to_string(), data: None, error: Some(e.to_string()), needs_manual_connect: None }
    };

    // 2. Write to a temporary file
    let temp_dir = std::env::temp_dir();
    let temp_path = temp_dir.join("video_clipper_clip.png");
    if let Err(e) = std::fs::write(&temp_path, bytes) {
        return ApiResponse { success: false, message: "".to_string(), data: None, error: Some(e.to_string()), needs_manual_connect: None };
    }

    let path_str = temp_path.to_string_lossy().to_string();

    // 3. Execute platform-specific clipboard command
    #[cfg(target_os = "macos")]
    {
        let script = format!("set the clipboard to (read (POSIX file \"{}\") as {{«class PNGf»}})", path_str);
        match Command::new("osascript").arg("-e").arg(&script).output() {
            Ok(out) => {
                if !out.status.success() {
                    let err = String::from_utf8_lossy(&out.stderr).to_string();
                    let _ = std::fs::remove_file(&temp_path);
                    return ApiResponse { success: false, message: "".to_string(), data: None, error: Some(err), needs_manual_connect: None };
                }
            }
            Err(e) => {
                let _ = std::fs::remove_file(&temp_path);
                return ApiResponse { success: false, message: "".to_string(), data: None, error: Some(e.to_string()), needs_manual_connect: None };
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        let ps_cmd = format!(
            "Add-Type -AssemblyName System.Windows.Forms; [System.Windows.Forms.Clipboard]::SetImage([System.Drawing.Image]::FromFile('{}'))",
            path_str
        );
        match Command::new("powershell").arg("-NoProfile").arg("-Command").arg(&ps_cmd).output() {
            Ok(out) => {
                if !out.status.success() {
                    let err = String::from_utf8_lossy(&out.stderr).to_string();
                    let _ = std::fs::remove_file(&temp_path);
                    return ApiResponse { success: false, message: "".to_string(), data: None, error: Some(err), needs_manual_connect: None };
                }
            }
            Err(e) => {
                let _ = std::fs::remove_file(&temp_path);
                return ApiResponse { success: false, message: "".to_string(), data: None, error: Some(e.to_string()), needs_manual_connect: None };
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Ok(mut child) = Command::new("xclip")
            .arg("-selection")
            .arg("clipboard")
            .arg("-t")
            .arg("image/png")
            .arg("-i")
            .arg(&path_str)
            .spawn() 
        {
            let _ = child.wait();
        }
    }

    // 4. Clean up temp file
    let _ = std::fs::remove_file(&temp_path);

    ApiResponse {
        success: true,
        message: "Copied to clipboard".to_string(),
        data: None,
        error: None,
        needs_manual_connect: None,
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            get_adb_status,
            adb_pair,
            adb_connect,
            adb_disconnect,
            load_video,
            start_recording,
            stop_recording,
            extract_frames,
            extract_frame,
            write_image_to_clipboard,
            save_image_to_disk
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_simulators() {
        let sims = get_ios_simulators();
        println!("TEST_SIMS: {:?}", sims);
    }
}
