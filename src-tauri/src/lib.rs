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
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Device {
    name: String,
    state: String,
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
    if let Ok(path_var) = std::env::var("PATH") {
        for path in std::env::split_paths(&path_var) {
            let exe_path = path.join(cmd);
            if exe_path.is_file() {
                return Some(exe_path);
            }
        }
    }
    None
}

fn find_adb() -> Option<PathBuf> {
    // 1. Try PATH
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



#[tauri::command]
fn get_adb_status() -> AdbStatus {
    let adb_path = match find_adb() {
        Some(path) => path,
        None => return AdbStatus { installed: false, devices: vec![] }
    };

    let output = match Command::new(&adb_path).arg("devices").output() {
        Ok(out) => out,
        Err(_) => return AdbStatus { installed: false, devices: vec![] }
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut devices = vec![];
    let mut connected_names = std::collections::HashSet::new();

    // Build a map of mDNS service name -> IP:Port to clean up cryptic adb-xxxx connection names
    let mut mdns_map = std::collections::HashMap::new();
    if let Ok(mdns_out) = Command::new(&adb_path).arg("mdns").arg("services").output() {
        let mdns_stdout = String::from_utf8_lossy(&mdns_out.stdout);
        for line in mdns_stdout.lines() {
            if line.contains("_adb-tls-connect") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 3 {
                    let mdns_name = parts[0].split('.').next().unwrap_or(parts[0]).trim().to_string();
                    let ip_port = parts[2].trim().to_string();
                    if !mdns_name.is_empty() && ip_port.contains(':') {
                        mdns_map.insert(mdns_name, ip_port);
                    }
                }
            }
        }
    }

    // Parse connected devices
    let lines: Vec<&str> = stdout.lines().collect();
    if lines.len() > 1 {
        for line in &lines[1..] {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 2 {
                let mut name = parts[0].trim().to_string();
                let state = parts[1].trim().to_string();
                if !name.is_empty() {
                    // Clean up cryptic mDNS auto-connect names (e.g. adb-xxxx._adb-tls-connect._tcp)
                    if name.contains("_adb-tls-connect") {
                        let base_name = name.split('.').next().unwrap_or(&name).trim();
                        let clean_base = base_name.split_whitespace().next().unwrap_or(base_name).to_string();
                        
                        if let Some(ip_port) = mdns_map.get(&clean_base) {
                            name = ip_port.clone();
                        } else {
                            // Fallback: extract hardware serial
                            let without_prefix = if clean_base.starts_with("adb-") {
                                &clean_base[4..]
                            } else {
                                &clean_base
                            };
                            let serial = without_prefix.split('-').next().unwrap_or(without_prefix);
                            name = format!("{} (Wireless)", serial);
                        }
                    }
                    
                    // Deduplicate connected names
                    if !connected_names.contains(&name) {
                        devices.push(Device { name: name.clone(), state });
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
                        devices.push(Device {
                            name: part.to_string(),
                            state: "available (paired)".to_string(),
                        });
                        connected_names.insert(part.to_string());
                        break;
                    }
                }
            }
        }
    }

    AdbStatus { installed: true, devices }
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

#[tauri::command]
fn stop_recording(device_id: Option<String>, state: State<'_, AppState>, _app: AppHandle) -> ApiResponse<String> {
    let adb_path = match find_adb() {
        Some(path) => path,
        None => return ApiResponse { success: false, message: "".to_string(), data: None, error: Some("ADB not installed".to_string()), needs_manual_connect: None }
    };

    let mut proc_lock = state.recording_process.lock().unwrap();
    let mut child = match proc_lock.take() {
        Some(c) => c,
        None => return ApiResponse { success: false, message: "".to_string(), data: None, error: Some("Not recording".to_string()), needs_manual_connect: None }
    };

    let _ = child.kill();
    let _ = child.wait();

    std::thread::sleep(Duration::from_secs(1));

    // Resolve local path to pull the video
    // Save to target directory
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
                message: "Recording saved".to_string(),
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

    let output = Command::new("ffmpeg")
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

    let output = Command::new("ffmpeg")
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
            extract_frame
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
