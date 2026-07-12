const { invoke } = window.__TAURI__.core;
const { getCurrentWindow } = window.__TAURI__.window;

// State variables
let activeDeviceId = null;
let isRecording = false;
let adbReady = false;
let checkAdbInterval = null;

// HTML Elements
const landingState = document.getElementById('landingState');
const editorState = document.getElementById('editorState');
const settingsModal = document.getElementById('settingsModal');
const landingError = document.getElementById('landingError');
const recordingStatus = document.getElementById('recordingStatus');
const btnRecord = document.getElementById('btnRecord');

// Video Clipper Elements & State
const videoPlayer = document.getElementById('videoPlayer');
const currentTimeDisplay = document.getElementById('currentTimeDisplay');
const totalTimeDisplay = document.getElementById('totalTimeDisplay');
const startTimeDisplay = document.getElementById('startTimeDisplay');
const endTimeDisplay = document.getElementById('endTimeDisplay');
const timelineSelected = document.getElementById('timelineSelected');
const videoNameDisplay = document.getElementById('videoNameDisplay');
const seekSlider = document.getElementById('seekSlider');
const btnPlayToggle = document.getElementById('btnPlayToggle');
const editorStatus = document.getElementById('editorStatus');
const editorError = document.getElementById('editorError');

// Add play/pause click handler on the video element itself
if (videoPlayer) {
    videoPlayer.addEventListener('click', togglePlayPause);
}

let startTime = 0;
let endTime = 0;
let currentFrameMode = false;

// Modal Elements
const adbInstalledArea = document.getElementById('adbInstalledArea');
const adbMissingArea = document.getElementById('adbMissingArea');
const installCommand = document.getElementById('installCommand');
const pairStatus = document.getElementById('pairStatus');
const savedDevicesList = document.getElementById('savedDevicesList');

// Listen to native Tauri file drop
try {
    getCurrentWindow().onDragDropEvent((event) => {
        if (event.payload.type === 'drop') {
            const files = event.payload.paths;
            if (files && files.length > 0) {
                const file = files[0];
                if (file.toLowerCase().endsWith('.mp4')) {
                    loadVideoPath(file);
                } else {
                    alert("Please drop an MP4 video file.");
                }
            }
        }
    });
} catch (e) {
    console.error("Failed to setup native drag-drop:", e);
}

// Restore saved video path on reload/HMR
document.addEventListener('DOMContentLoaded', () => {
    const savedPath = localStorage.getItem('currentVideoPath');
    if (savedPath) {
        loadVideoPath(savedPath);
    }
    const settingsOpen = localStorage.getItem('settingsOpen');
    if (settingsOpen === 'true') {
        openSettings();
    }
});

// Drag & drop highlight for drop-zone
const dropZone = document.getElementById('dropZone');
dropZone.addEventListener('dragover', (e) => {
    e.preventDefault();
    dropZone.classList.add('dragover');
});
dropZone.addEventListener('dragleave', () => {
    dropZone.classList.remove('dragover');
});
dropZone.addEventListener('drop', (e) => {
    e.preventDefault();
    dropZone.classList.remove('dragover');
});

function handleFileSelect(event) {
    const file = event.target.files[0];
    if (file) {
        // In Tauri, File object contains the absolute path in .path
        const path = file.path || file.name;
        loadVideoPath(path);
    }
}

// Time formatting helper: MM:SS.mmm
function formatTime(seconds) {
    if (isNaN(seconds) || seconds === null || seconds === undefined) return "00:00.000";
    const mins = Math.floor(seconds / 60);
    const secs = Math.floor(seconds % 60);
    const ms = Math.floor((seconds % 1) * 1000);
    return `${String(mins).padStart(2, '0')}:${String(secs).padStart(2, '0')}.${String(ms).padStart(3, '0')}`;
}

// Load Video Path
async function loadVideoPath(path) {
    try {
        const response = await invoke('load_video', { path });
        if (response.success) {
            // Save path to restore on HMR/reloads
            localStorage.setItem('currentVideoPath', path);
            
            // Update video element source
            const assetUrl = window.__TAURI__.core.convertFileSrc(path);
            videoPlayer.src = assetUrl;
            if (videoNameDisplay) {
                videoNameDisplay.innerText = path.split('/').pop();
            }
            
            // Set path details card
            const srcDisplay = document.getElementById('fullVideoPathDisplay');
            srcDisplay.innerText = path;
            srcDisplay.title = path;
            updateOutputPathDisplay();
            
            // Show editor and hide settings button
            btnSettings.style.display = 'none';
            landingState.style.display = 'none';
            editorState.style.display = 'flex';
            
            // Wait for video to load metadata
            videoPlayer.onloadedmetadata = () => {
                startTime = 0;
                endTime = videoPlayer.duration;
                startTimeDisplay.innerText = formatTime(0);
                endTimeDisplay.innerText = formatTime(videoPlayer.duration);
                totalTimeDisplay.innerText = formatTime(videoPlayer.duration);
                seekSlider.value = 0;
                updateTimelineBar();
            };
            
            videoPlayer.ontimeupdate = () => {
                currentTimeDisplay.innerText = formatTime(videoPlayer.currentTime);
                if (videoPlayer.duration) {
                    seekSlider.value = (videoPlayer.currentTime / videoPlayer.duration) * 100;
                }
                updateTimelineBar();
            };

            videoPlayer.onended = () => {
                btnPlayToggle.innerText = "Play";
            };
        } else {
            alert("Error loading video: " + response.error);
        }
    } catch (e) {
        alert("Failed to load video: " + e);
    }
}

// Update the output folder path display
function updateOutputPathDisplay() {
    const videoPath = document.getElementById('fullVideoPathDisplay').innerText;
    if (!videoPath || videoPath === "Loading...") return;
    const parts = videoPath.split('/');
    const filename = parts.pop();
    const dir = parts.join('/');
    const stem = filename.substring(0, filename.lastIndexOf('.')) || filename;
    const fps = getFPS().toString();
    const targetFolder = `${dir}/${stem}_frames_${fps}fps/`;
    const targetDisplay = document.getElementById('outputPathDisplay');
    targetDisplay.innerText = targetFolder;
    targetDisplay.title = targetFolder;
}

function updateTimelineBar() {
    if (!videoPlayer.duration) return;
    const pctStart = (startTime / videoPlayer.duration) * 100;
    const pctEnd = (endTime / videoPlayer.duration) * 100;
    
    timelineSelected.style.left = `${pctStart}%`;
    timelineSelected.style.width = `${pctEnd - pctStart}%`;
}

function seekVideo(val) {
    if (videoPlayer.duration) {
        videoPlayer.currentTime = (parseFloat(val) / 100) * videoPlayer.duration;
    }
}

function togglePlayPause() {
    if (videoPlayer.paused) {
        videoPlayer.play();
        btnPlayToggle.innerText = "Pause";
    } else {
        videoPlayer.pause();
        btnPlayToggle.innerText = "Play";
    }
}

function getFPS() {
    let fpsVal = parseInt(document.getElementById('fpsInput').value) || 60;
    if (fpsVal < 1) fpsVal = 1;
    if (fpsVal > 320) fpsVal = 320;
    document.getElementById('fpsInput').value = fpsVal;
    return fpsVal;
}

function toggleCurrentFrameMode(checked) {
    currentFrameMode = checked;
    const fpsContainer = document.getElementById('fpsInputContainer');
    if (checked) {
        fpsContainer.classList.add('disabled');
    } else {
        fpsContainer.classList.remove('disabled');
    }
    updateOutputPathDisplay();
}

function handleSaveAction() {
    if (currentFrameMode) {
        extractSingleFrame();
    } else {
        extractAllFrames();
    }
}

function handleCopyToClipboard() {
    if (currentFrameMode) {
        copyCurrentFrameToClipboard();
    } else {
        copyFramesGridToClipboard();
    }
}

function setCurrentAsStart() {
    startTime = videoPlayer.currentTime;
    if (startTime > endTime) {
        endTime = startTime;
        endTimeDisplay.innerText = formatTime(endTime);
    }
    startTimeDisplay.innerText = formatTime(startTime);
    updateTimelineBar();
}

// Global hook for key events
document.addEventListener('keydown', (e) => {
    if (document.activeElement.tagName === 'INPUT' || (settingsModal && settingsModal.style.display === 'flex')) return;
    
    if (e.code === 'Space') {
        e.preventDefault();
        togglePlayPause();
    } else if (e.code === 'ArrowLeft') {
        e.preventDefault();
        videoPlayer.currentTime = Math.max(0, videoPlayer.currentTime - 1);
    } else if (e.code === 'ArrowRight') {
        e.preventDefault();
        videoPlayer.currentTime = Math.min(videoPlayer.duration, videoPlayer.currentTime + 1);
    } else if (e.code === 'Comma') {
        e.preventDefault();
        stepFrame(-1);
    } else if (e.code === 'Period') {
        e.preventDefault();
        stepFrame(1);
    }
});

function setCurrentAsEnd() {
    endTime = videoPlayer.currentTime;
    if (endTime < startTime) {
        startTime = endTime;
        startTimeDisplay.innerText = formatTime(startTime);
    }
    endTimeDisplay.innerText = formatTime(endTime);
    updateTimelineBar();
}

function stepFrame(frames) {
    const fps = getFPS();
    videoPlayer.currentTime += frames * (1 / fps);
}

// Go Home
function goHome() {
    videoPlayer.src = "";
    localStorage.removeItem('currentVideoPath');
    btnSettings.style.display = 'block';
    editorState.style.display = 'none';
    landingState.style.display = 'flex';
}

// Extract Frames
async function extractAllFrames() {
    const start = startTime;
    const end = endTime;
    const fps = getFPS().toString();
    
    editorStatus.style.display = 'block';
    editorStatus.innerText = 'Extracting frames...';
    editorError.style.display = 'none';
    
    try {
        const res = await invoke('extract_frames', { start, end, fps });
        if (res.success) {
            editorStatus.innerText = res.message;
        } else {
            editorStatus.style.display = 'none';
            editorError.style.display = 'block';
            editorError.innerText = res.error;
        }
    } catch (e) {
        editorStatus.style.display = 'none';
        editorError.style.display = 'block';
        editorError.innerText = e;
    }
}

async function extractSingleFrame() {
    const time = videoPlayer.currentTime;
    
    editorStatus.style.display = 'block';
    editorStatus.innerText = 'Extracting frame...';
    editorError.style.display = 'none';
    
    try {
        const res = await invoke('extract_frame', { time });
        if (res.success) {
            editorStatus.innerText = res.message;
        } else {
            editorStatus.style.display = 'none';
            editorError.style.display = 'block';
            editorError.innerText = res.error;
        }
    } catch (e) {
        editorStatus.style.display = 'none';
        editorError.style.display = 'block';
        editorError.innerText = e;
    }
}

async function copyCurrentFrameToClipboard() {
    const btn = document.getElementById('btnCopyToClipboard');
    const oldText = btn.innerText;
    btn.innerHTML = '<span class="loader" style="width:10px; height:10px; border-width:1px; margin-right:4px;"></span>Copying...';
    btn.disabled = true;
    
    editorStatus.style.display = 'block';
    editorStatus.innerText = 'Copying current frame...';
    editorError.style.display = 'none';
    
    try {
        const canvas = document.createElement('canvas');
        canvas.width = videoPlayer.videoWidth;
        canvas.height = videoPlayer.videoHeight;
        const ctx = canvas.getContext('2d');
        ctx.drawImage(videoPlayer, 0, 0, canvas.width, canvas.height);
        
        canvas.toBlob(async (blob) => {
            if (!blob) {
                throw new Error("Failed to create blob from video frame");
            }
            try {
                await navigator.clipboard.write([
                    new ClipboardItem({ [blob.type]: blob })
                ]);
                editorStatus.innerText = 'Current frame copied to clipboard!';
            } catch (err) {
                console.error("Clipboard API failed:", err);
                editorStatus.style.display = 'none';
                editorError.style.display = 'block';
                editorError.innerText = 'Clipboard error: ' + err;
            } finally {
                btn.innerText = oldText;
                btn.disabled = false;
            }
        }, 'image/png');
    } catch (e) {
        editorStatus.style.display = 'none';
        editorError.style.display = 'block';
        editorError.innerText = e.message || e;
        btn.innerText = oldText;
        btn.disabled = false;
    }
}

async function copyFramesGridToClipboard() {
    const start = startTime;
    const end = endTime;
    const fps = getFPS();
    const btn = document.getElementById('btnCopyToClipboard');
    
    const duration = end - start;
    if (duration <= 0) {
        editorError.style.display = 'block';
        editorError.innerText = 'Invalid start/end points. Duration must be positive.';
        return;
    }
    
    const frameCount = Math.max(1, Math.round(duration * fps));
    if (frameCount > 120) {
        if (!confirm(`Generating a grid with ${frameCount} frames might take a while. Do you want to proceed?`)) {
            return;
        }
    }
    
    const oldText = btn.innerText;
    btn.innerHTML = '<span class="loader" style="width:10px; height:10px; border-width:1px; margin-right:4px;"></span>Generating Grid...';
    btn.disabled = true;
    
    editorStatus.style.display = 'block';
    editorStatus.innerText = `Generating grid of ${frameCount} frames (${fps} fps)...`;
    editorError.style.display = 'none';
    
    try {
        const wasPlaying = !videoPlayer.paused;
        if (wasPlaying) videoPlayer.pause();
        const originalTime = videoPlayer.currentTime;
        
        // Define grid geometry
        const cols = Math.ceil(Math.sqrt(frameCount));
        const rows = Math.ceil(frameCount / cols);
        
        // Scale down frame size in grid to keep it memory efficient
        const frameWidth = 240;
        const frameHeight = Math.round(frameWidth * (videoPlayer.videoHeight / videoPlayer.videoWidth));
        
        const gridCanvas = document.createElement('canvas');
        gridCanvas.width = cols * frameWidth;
        gridCanvas.height = rows * frameHeight;
        const gridCtx = gridCanvas.getContext('2d');
        
        // Background color
        gridCtx.fillStyle = '#1e1e24';
        gridCtx.fillRect(0, 0, gridCanvas.width, gridCanvas.height);
        
        // Seek & draw sync helper
        const seekToTime = (time) => {
            return new Promise((resolve) => {
                const onSeeked = () => {
                    videoPlayer.removeEventListener('seeked', onSeeked);
                    resolve();
                };
                videoPlayer.addEventListener('seeked', onSeeked);
                videoPlayer.currentTime = time;
            });
        };
        
        for (let i = 0; i < frameCount; i++) {
            const t = start + (i / fps);
            if (t > end) break;
            
            await seekToTime(t);
            
            const col = i % cols;
            const row = Math.floor(i / cols);
            const x = col * frameWidth;
            const y = row * frameHeight;
            
            gridCtx.drawImage(videoPlayer, x, y, frameWidth, frameHeight);
            
            // Draw a neat label with time offsets and frame numbers
            gridCtx.fillStyle = 'rgba(0, 0, 0, 0.6)';
            gridCtx.fillRect(x + 4, y + 4, 75, 18);
            
            gridCtx.fillStyle = '#ffffff';
            gridCtx.font = '10px -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif';
            gridCtx.fillText(`#${i+1} (+${(t - start).toFixed(3)}s)`, x + 8, y + 16);
        }
        
        // Copy to clipboard
        gridCanvas.toBlob(async (blob) => {
            if (!blob) {
                throw new Error("Failed to create blob from grid canvas");
            }
            try {
                await navigator.clipboard.write([
                    new ClipboardItem({ [blob.type]: blob })
                ]);
                editorStatus.innerText = `Grid of ${frameCount} frames copied to clipboard!`;
            } catch (err) {
                console.error("Clipboard API failed:", err);
                editorStatus.style.display = 'none';
                editorError.style.display = 'block';
                editorError.innerText = 'Clipboard error: ' + err;
            } finally {
                // Restore state
                videoPlayer.currentTime = originalTime;
                if (wasPlaying) videoPlayer.play();
                
                btn.innerText = oldText;
                btn.disabled = false;
            }
        }, 'image/png');
        
    } catch (e) {
        editorStatus.style.display = 'none';
        editorError.style.display = 'block';
        editorError.innerText = e.message || e;
        btn.innerText = oldText;
        btn.disabled = false;
    }
}

// ADB / Recording Controls
async function toggleRecording() {
    if (isRecording) {
        // Stop recording
        btnRecord.innerHTML = '<span class="loader"></span> Loading video...';
        btnRecord.classList.remove('recording-active');
        btnRecord.disabled = true;
        
        // Let the browser paint the loading state first
        setTimeout(async () => {
            try {
                const res = await invoke('stop_recording', { deviceId: activeDeviceId });
                if (res.success) {
                    isRecording = false;
                    btnRecord.innerHTML = '📱 Record Android Screen (ADB)';
                    btnRecord.classList.remove('recording-active');
                    btnRecord.disabled = false;
                    recordingStatus.style.display = 'none';
                    
                    // Load pulled video
                    loadVideoPath(res.data);
                } else {
                    throw new Error(res.error);
                }
            } catch (e) {
                landingError.style.display = 'block';
                landingError.innerText = e.message || e;
                resetRecordButton();
            }
        }, 50);
    } else {
        // Pre-flight check
        try {
            const status = await invoke('get_adb_status');
            if (!status.installed || status.devices.length === 0) {
                openSettings();
                return;
            }
            
            if (!activeDeviceId && status.devices.length > 0) {
                activeDeviceId = status.devices[0].name;
            }
            
            btnRecord.innerHTML = '<span class="loader"></span> Starting record...';
            btnRecord.classList.remove('recording-active');
            btnRecord.disabled = true;
            
            recordingStatus.style.display = 'block';
            recordingStatus.innerText = 'Preparing screen recording...';
            
            const res = await invoke('start_recording', { deviceId: activeDeviceId });
            if (!res.success) {
                throw new Error(res.error);
            }
            
            isRecording = true;
            btnRecord.innerHTML = '⏹ Stop & Load Video';
            btnRecord.classList.add('recording-active');
            btnRecord.disabled = false;
            recordingStatus.innerText = 'Recording in progress... Click above to stop';
        } catch (e) {
            landingError.style.display = 'block';
            landingError.innerText = e.message || e;
            resetRecordButton();
        }
    }
}

function resetRecordButton() {
    isRecording = false;
    btnRecord.innerHTML = '📱 Record Android Screen (ADB)';
    btnRecord.classList.remove('recording-active');
    btnRecord.disabled = false;
    recordingStatus.style.display = 'none';
}

// Modal Settings
function openSettings() {
    settingsModal.style.display = 'flex';
    localStorage.setItem('settingsOpen', 'true');
    checkAdbStatus();
    if (!checkAdbInterval) {
        checkAdbInterval = setInterval(checkAdbStatus, 3000);
    }
}

function closeSettings() {
    settingsModal.style.display = 'none';
    localStorage.removeItem('settingsOpen');
    if (checkAdbInterval) {
        clearInterval(checkAdbInterval);
        checkAdbInterval = null;
    }
}

async function checkAdbStatus() {
    try {
        const status = await invoke('get_adb_status');
        if (status.installed) {
            adbInstalledArea.style.display = 'block';
            adbMissingArea.style.display = 'none';
            adbReady = true;
            
            // Save active element details if it's one of our port inputs to prevent focus loss during 3s refreshes
            let activeInputId = null;
            let activeInputValue = '';
            let activeSelectionStart = 0;
            let activeSelectionEnd = 0;
            if (document.activeElement && document.activeElement.classList.contains('device-port-input')) {
                activeInputId = document.activeElement.id;
                activeInputValue = document.activeElement.value;
                activeSelectionStart = document.activeElement.selectionStart;
                activeSelectionEnd = document.activeElement.selectionEnd;
            }
            
            savedDevicesList.innerHTML = '';
            
            // Get saved IPs from localStorage
            let savedIps = JSON.parse(localStorage.getItem('savedDeviceIps') || '[]');
            
            // Split out connected vs available (paired) devices
            const connectedDevices = status.devices.filter(d => d.state === 'device');
            const availableDevices = status.devices.filter(d => d.state === 'available (paired)');
            
            // Auto-select active device if not selected
            if (connectedDevices.length > 0) {
                if (!activeDeviceId || !connectedDevices.find(d => d.name === activeDeviceId)) {
                    activeDeviceId = connectedDevices[0].name;
                }
            } else {
                activeDeviceId = null;
            }
            
            // Track rendering to avoid duplicates
            const renderedIps = new Set();
            
            // 1. Render currently connected devices (green dot, Connected badge)
            connectedDevices.forEach(d => {
                const item = document.createElement('div');
                item.className = `saved-device-item ${d.name === activeDeviceId ? 'active' : ''}`;
                
                const info = document.createElement('div');
                info.className = 'device-info';
                info.onclick = () => {
                    activeDeviceId = d.name;
                    checkAdbStatus();
                };
                
                const dot = document.createElement('span');
                dot.className = 'device-status-dot connected';
                
                const name = document.createElement('span');
                name.className = 'device-name';
                name.innerText = d.name;
                name.title = d.name;
                
                const badge = document.createElement('span');
                badge.className = 'device-badge connected';
                badge.innerText = 'Connected';
                
                info.appendChild(dot);
                info.appendChild(name);
                info.appendChild(badge);
                item.appendChild(info);
                
                const actions = document.createElement('div');
                actions.className = 'device-actions';
                
                // Show Disconnect button for wireless connections
                if (d.name.includes(':') || d.name.includes('_adb-tls-connect')) {
                    const btnDisc = document.createElement('button');
                    btnDisc.className = 'btn-device-action disconnect';
                    btnDisc.innerText = 'Disconnect';
                    btnDisc.onclick = (e) => {
                        e.stopPropagation();
                        disconnectSavedDevice(d.name, btnDisc);
                    };
                    actions.appendChild(btnDisc);
                } else {
                    const label = document.createElement('span');
                    label.style.color = 'var(--text-muted)';
                    label.style.fontSize = '12px';
                    label.style.marginRight = '8px';
                    label.innerText = 'USB';
                    actions.appendChild(label);
                }
                
                item.appendChild(actions);
                savedDevicesList.appendChild(item);
                
                // Auto-save IP if wireless
                if (d.name.includes(':')) {
                    const ip = d.name.split(':')[0];
                    renderedIps.add(ip);
                    if (ip && !savedIps.includes(ip)) {
                        savedIps.push(ip);
                        localStorage.setItem('savedDeviceIps', JSON.stringify(savedIps));
                    }
                }
            });
            
            // 2. Render available devices discovered via mDNS (blue dot, Available badge)
            availableDevices.forEach(d => {
                const ip = d.name.includes(':') ? d.name.split(':')[0] : d.name;
                renderedIps.add(ip);
                
                const item = document.createElement('div');
                item.className = 'saved-device-item';
                
                const info = document.createElement('div');
                info.className = 'device-info';
                
                const dot = document.createElement('span');
                dot.className = 'device-status-dot available';
                
                const name = document.createElement('span');
                name.className = 'device-name';
                name.innerText = d.name;
                name.title = d.name;
                
                const badge = document.createElement('span');
                badge.className = 'device-badge available';
                badge.innerText = 'Available';
                
                info.appendChild(dot);
                info.appendChild(name);
                info.appendChild(badge);
                item.appendChild(info);
                
                const actions = document.createElement('div');
                actions.className = 'device-actions';
                
                const btnConn = document.createElement('button');
                btnConn.className = 'btn-device-action';
                btnConn.innerText = 'Connect';
                btnConn.onclick = () => connectSavedDeviceDirect(d.name, btnConn);
                
                actions.appendChild(btnConn);
                item.appendChild(actions);
                savedDevicesList.appendChild(item);
                
                // Ensure IP is in saved list
                if (ip && !savedIps.includes(ip)) {
                    savedIps.push(ip);
                    localStorage.setItem('savedDeviceIps', JSON.stringify(savedIps));
                }
            });
            
            // 3. Render paired/saved devices offline (grey dot, Paired badge)
            savedIps.forEach(ip => {
                // If already shown as connected or available, skip
                if (renderedIps.has(ip)) return;
                
                const item = document.createElement('div');
                item.className = 'saved-device-item';
                
                const info = document.createElement('div');
                info.className = 'device-info';
                
                const dot = document.createElement('span');
                dot.className = 'device-status-dot';
                
                const name = document.createElement('span');
                name.className = 'device-name';
                name.innerText = ip;
                name.title = ip;
                
                const badge = document.createElement('span');
                badge.className = 'device-badge paired';
                badge.innerText = 'Paired';
                
                info.appendChild(dot);
                info.appendChild(name);
                info.appendChild(badge);
                item.appendChild(info);
                
                const actions = document.createElement('div');
                actions.className = 'device-actions';
                
                const portInput = document.createElement('input');
                portInput.type = 'text';
                portInput.placeholder = 'Port';
                portInput.className = 'device-port-input';
                portInput.id = `port_${ip.replace(/\./g, '_')}`;
                
                if (portInput.id === activeInputId) {
                    portInput.value = activeInputValue;
                }
                
                portInput.onkeydown = (e) => {
                    if (e.key === 'Enter') {
                        connectSavedDevice(ip, portInput.id, btnConn);
                    }
                };
                
                const btnConn = document.createElement('button');
                btnConn.className = 'btn-device-action';
                btnConn.innerText = 'Connect';
                btnConn.onclick = () => connectSavedDevice(ip, portInput.id, btnConn);
                
                const btnForget = document.createElement('button');
                btnForget.className = 'btn-device-forget';
                btnForget.innerHTML = '✕';
                btnForget.title = 'Forget Device';
                btnForget.onclick = () => forgetSavedDevice(ip);
                
                actions.appendChild(portInput);
                actions.appendChild(btnConn);
                actions.appendChild(btnForget);
                item.appendChild(actions);
                
                savedDevicesList.appendChild(item);
            });
            
            if (savedDevicesList.children.length === 0) {
                savedDevicesList.innerHTML = '<div class="list-placeholder">No devices connected or saved</div>';
            }
            
            // Restore focus and cursor selection
            if (activeInputId) {
                const restoredInput = document.getElementById(activeInputId);
                if (restoredInput) {
                    restoredInput.focus();
                    try {
                        restoredInput.setSelectionRange(activeSelectionStart, activeSelectionEnd);
                    } catch (e) {}
                }
            }
        } else {
            adbInstalledArea.style.display = 'none';
            adbMissingArea.style.display = 'block';
            adbReady = false;
        }
    } catch (e) {
        console.error("Failed to check ADB status:", e);
    }
}

async function connectSavedDeviceDirect(ipPort, btn) {
    const oldText = btn.innerText;
    btn.innerHTML = '<span class="loader" style="width:10px; height:10px; border-width:1px; margin-right:4px;"></span>Connecting...';
    btn.disabled = true;
    
    try {
        const res = await invoke('adb_connect', { ipPort });
        if (res.success) {
            pairStatus.innerText = "Connected successfully!";
            pairStatus.style.color = "var(--success-color)";
            checkAdbStatus();
        } else {
            alert("Connection failed: " + res.error);
        }
    } catch (e) {
        alert("Connection error: " + (e.message || e));
    } finally {
        btn.innerText = oldText;
        btn.disabled = false;
    }
}

async function connectSavedDevice(ip, portInputId, btn) {
    const portInput = document.getElementById(portInputId);
    const port = portInput.value.trim();
    if (!port) {
        portInput.classList.add('error');
        portInput.focus();
        portInput.addEventListener('input', () => {
            portInput.classList.remove('error');
        }, { once: true });
        return;
    }
    
    const ipPort = `${ip}:${port}`;
    const oldText = btn.innerText;
    btn.innerHTML = '<span class="loader" style="width:10px; height:10px; border-width:1px; margin-right:4px;"></span>Connecting...';
    btn.disabled = true;
    
    try {
        const res = await invoke('adb_connect', { ipPort });
        if (res.success) {
            pairStatus.innerText = "Connected successfully!";
            pairStatus.style.color = "var(--success-color)";
            portInput.value = '';
            checkAdbStatus();
        } else {
            alert("Connection failed: " + res.error);
        }
    } catch (e) {
        alert("Connection error: " + (e.message || e));
    } finally {
        btn.innerText = oldText;
        btn.disabled = false;
    }
}

async function disconnectSavedDevice(ipPort, btn) {
    const oldText = btn.innerText;
    btn.innerHTML = '<span class="loader" style="width:10px; height:10px; border-width:1px; margin-right:4px;"></span>...';
    btn.disabled = true;
    
    try {
        const res = await invoke('adb_disconnect', { ipPort });
        if (res.success) {
            pairStatus.innerText = "Disconnected.";
            pairStatus.style.color = "var(--text-muted)";
            checkAdbStatus();
        } else {
            alert("Disconnect failed: " + res.error);
        }
    } catch (e) {
        alert("Disconnect error: " + (e.message || e));
    } finally {
        btn.innerText = oldText;
        btn.disabled = false;
    }
}

function forgetSavedDevice(ip) {
    if (confirm(`Forget saved device IP ${ip}?`)) {
        let savedIps = JSON.parse(localStorage.getItem('savedDeviceIps') || '[]');
        savedIps = savedIps.filter(item => item !== ip);
        localStorage.setItem('savedDeviceIps', JSON.stringify(savedIps));
        checkAdbStatus();
    }
}

// ADB Wireless pairing & connect
async function pairAdb() {
    let ip = document.getElementById('adbIp').value.trim();
    let code = document.getElementById('adbCode').value.trim();
    const btn = document.getElementById('btnPair');
    
    if (!ip) {
        pairStatus.innerText = "Please enter the IP:PORT.";
        pairStatus.style.color = "var(--danger-color)";
        return;
    }
    
    // Auto-detect swapped inputs
    if (code && ip.length === 6 && !ip.includes(':') && code.includes(':')) {
        const temp = ip;
        ip = code;
        code = temp;
        document.getElementById('adbIp').value = ip;
        document.getElementById('adbCode').value = code;
    }
    
    btn.innerHTML = '<span class="loader"></span> Connecting...';
    btn.disabled = true;
    pairStatus.innerText = "";
    
    try {
        if (!code) {
            // Direct Connect
            const res = await invoke('adb_connect', { ipPort: ip });
            if (res.success) {
                pairStatus.innerText = "Connected!";
                pairStatus.style.color = "var(--success-color)";
                setTimeout(() => {
                    checkAdbStatus();
                    btn.innerHTML = 'Connect / Pair';
                    btn.disabled = false;
                    document.getElementById('adbIp').value = '';
                }, 1000);
            } else {
                throw new Error(res.error);
            }
        } else {
            // Pair & Auto Connect
            const res = await invoke('adb_pair', { ipPort: ip, code });
            if (res.success) {
                pairStatus.innerText = res.message;
                pairStatus.style.color = "var(--success-color)";
                btn.innerHTML = 'Connect / Pair';
                btn.disabled = false;
                document.getElementById('adbCode').value = '';
                
                // Save IP to saved device list
                const ipPart = ip.split(':')[0];
                if (ipPart) {
                    let savedIps = JSON.parse(localStorage.getItem('savedDeviceIps') || '[]');
                    if (!savedIps.includes(ipPart)) {
                        savedIps.push(ipPart);
                        localStorage.setItem('savedDeviceIps', JSON.stringify(savedIps));
                    }
                }
                
                document.getElementById('adbIp').value = '';
                checkAdbStatus();
            } else {
                throw new Error(res.error);
            }
        }
    } catch (e) {
        pairStatus.innerText = e.message || e;
        pairStatus.style.color = "var(--danger-color)";
        btn.innerHTML = 'Connect / Pair';
        btn.disabled = false;
    }
}

// Commands tabs for instructions
const installCommands = {
    mac: "brew install android-platform-tools",
    win: "choco install adb",
    linux: "sudo apt update && sudo apt install android-tools-adb android-tools-fastboot"
};

function switchTab(os) {
    document.querySelectorAll('.code-tab').forEach(t => t.classList.remove('active'));
    event.target.classList.add('active');
    installCommand.innerText = installCommands[os];
}

function copyInstallCmd() {
    navigator.clipboard.writeText(installCommand.innerText);
    const btn = event.target;
    btn.innerText = "Copied!";
    setTimeout(() => { btn.innerText = "Copy"; }, 2000);
}

// Keyboard shortcuts
window.addEventListener('keydown', (e) => {
    // If inside input, ignore shortcuts
    if (document.activeElement.tagName === 'INPUT') return;
    
    if (e.code === 'Space') {
        e.preventDefault();
        if (videoPlayer.paused) videoPlayer.play();
        else videoPlayer.pause();
    } else if (e.code === 'KeyI') {
        setCurrentAsStart();
    } else if (e.code === 'KeyO') {
        setCurrentAsEnd();
    } else if (e.code === 'ArrowLeft') {
        stepFrame(-1);
    } else if (e.code === 'ArrowRight') {
        stepFrame(1);
    }
});

// Copy path text to clipboard
async function copyPath(elementId) {
    const text = document.getElementById(elementId).innerText;
    try {
        await navigator.clipboard.writeText(text);
        
        // Find copy button next to value element
        const valElem = document.getElementById(elementId);
        const btn = valElem.nextElementSibling || valElem.parentElement.querySelector('.btn-copy-path');
        if (btn) {
            const originalColor = btn.style.color;
            btn.style.color = '#10b981'; // Green color for success
            setTimeout(() => {
                btn.style.color = originalColor;
            }, 1000);
        }
    } catch (err) {
        console.error('Failed to copy text: ', err);
    }
}
