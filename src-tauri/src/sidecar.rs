//! Sidecar management for the .NET Text Monitor service.
//!
//! This module handles starting and stopping the text-monitor sidecar
//! that provides global text selection detection via Windows hooks.

use std::sync::Arc;
use tauri::async_runtime::Mutex;
use tauri::AppHandle;
use tauri_plugin_shell::process::CommandChild;
use tauri_plugin_shell::ShellExt;

#[cfg(target_os = "windows")]
use std::sync::Once;

#[cfg(target_os = "windows")]
static JOB_INIT: Once = Once::new();

/// Initialize a Windows Job Object that will kill all child processes when this process exits.
/// This ensures the sidecar is terminated even on Ctrl+C or abnormal termination.
#[cfg(target_os = "windows")]
pub fn init_job_object() {
    JOB_INIT.call_once(|| {
        use std::ptr;

        #[link(name = "kernel32")]
        extern "system" {
            fn CreateJobObjectW(lpJobAttributes: *mut std::ffi::c_void, lpName: *const u16) -> *mut std::ffi::c_void;
            fn SetInformationJobObject(hJob: *mut std::ffi::c_void, JobObjectInformationClass: u32, lpJobObjectInformation: *mut std::ffi::c_void, cbJobObjectInformationLength: u32) -> i32;
            fn AssignProcessToJobObject(hJob: *mut std::ffi::c_void, hProcess: *mut std::ffi::c_void) -> i32;
            fn GetCurrentProcess() -> *mut std::ffi::c_void;
        }

        const JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE: u32 = 0x2000;
        const JOB_OBJECT_EXTENDED_LIMIT_INFORMATION: u32 = 9;

        #[repr(C)]
        struct JOBOBJECT_BASIC_LIMIT_INFORMATION {
            per_process_user_time_limit: i64,
            per_job_user_time_limit: i64,
            limit_flags: u32,
            minimum_working_set_size: usize,
            maximum_working_set_size: usize,
            active_process_limit: u32,
            affinity: usize,
            priority_class: u32,
            scheduling_class: u32,
        }

        #[repr(C)]
        struct IO_COUNTERS {
            read_operation_count: u64,
            write_operation_count: u64,
            other_operation_count: u64,
            read_transfer_count: u64,
            write_transfer_count: u64,
            other_transfer_count: u64,
        }

        #[repr(C)]
        struct JOBOBJECT_EXTENDED_LIMIT_INFORMATION {
            basic_limit_information: JOBOBJECT_BASIC_LIMIT_INFORMATION,
            io_info: IO_COUNTERS,
            process_memory_limit: usize,
            job_memory_limit: usize,
            peak_process_memory_used: usize,
            peak_job_memory_used: usize,
        }

        unsafe {
            let job = CreateJobObjectW(ptr::null_mut(), ptr::null());
            if job.is_null() {
                eprintln!("[Sidecar] Failed to create job object");
                return;
            }

            let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            info.basic_limit_information.limit_flags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;

            let result = SetInformationJobObject(
                job,
                JOB_OBJECT_EXTENDED_LIMIT_INFORMATION,
                &mut info as *mut _ as *mut std::ffi::c_void,
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            );

            if result == 0 {
                eprintln!("[Sidecar] Failed to set job object information");
                return;
            }

            let result = AssignProcessToJobObject(job, GetCurrentProcess());
            if result == 0 {
                eprintln!("[Sidecar] Failed to assign process to job object");
                return;
            }

            println!("[Sidecar] Job object initialized - child processes will terminate on exit");
        }
    });
}

#[cfg(not(target_os = "windows"))]
pub fn init_job_object() {
    // No-op on non-Windows platforms
}

/// Kill any existing text-monitor processes to ensure only one instance runs.
/// This is called before starting a new sidecar instance.
#[cfg(target_os = "windows")]
fn kill_existing_text_monitor() {
    use std::process::Command;
    use std::os::windows::process::CommandExt;

    // Kill both possible naming conventions
    let names = [
        "text-monitor-x86_64-pc-windows-msvc.exe",
        "text-monitor.exe",
    ];

    // CREATE_NO_WINDOW flag to prevent console window from appearing
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    for name in names {
        let result = Command::new("taskkill")
            .args(["/F", "/IM", name])
            .creation_flags(CREATE_NO_WINDOW)
            .output();

        if let Ok(output) = result {
            if output.status.success() {
                println!("[Sidecar] Killed existing process: {}", name);
            }
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn kill_existing_text_monitor() {
    // On non-Windows, use pkill
    use std::process::Command;
    let _ = Command::new("pkill")
        .args(["-f", "text-monitor"])
        .output();
}

/// State for managing the sidecar process.
pub struct SidecarState {
    /// The running sidecar child process, if any.
    child: Arc<Mutex<Option<CommandChild>>>,
}

impl SidecarState {
    /// Creates a new SidecarState.
    pub fn new() -> Self {
        Self {
            child: Arc::new(Mutex::new(None)),
        }
    }

    /// Starts the text-monitor sidecar if not already running.
    ///
    /// # Arguments
    /// * `app` - The Tauri app handle
    ///
    /// # Returns
    /// * `Ok(())` if the sidecar started successfully or was already running
    /// * `Err(String)` if the sidecar failed to start
    pub async fn start(&self, app: &AppHandle) -> Result<(), String> {
        let mut guard = self.child.lock().await;

        // Check if already running in our state
        if guard.is_some() {
            log::info!("Sidecar: Text monitor already running");
            println!("[Sidecar] Text monitor already running");
            return Ok(());
        }

        // Kill any existing text-monitor processes (from previous runs)
        kill_existing_text_monitor();

        log::info!("Sidecar: Starting text-monitor...");
        println!("[Sidecar] Starting text-monitor...");

        // Spawn the sidecar using shell plugin
        let sidecar_command = match app.shell().sidecar("text-monitor") {
            Ok(cmd) => cmd,
            Err(e) => {
                let msg = format!("Failed to create sidecar command: {}", e);
                log::error!("Sidecar: {}", msg);
                println!("[Sidecar Error] {}", msg);
                return Err(msg);
            }
        };

        let (mut rx, child) = match sidecar_command.spawn() {
            Ok(result) => result,
            Err(e) => {
                let msg = format!("Failed to spawn sidecar: {}", e);
                log::error!("Sidecar: {}", msg);
                println!("[Sidecar Error] {}", msg);
                return Err(msg);
            }
        };

        // Store the child process
        *guard = Some(child);

        log::info!("Sidecar: Text monitor started successfully");
        println!("[Sidecar] Text monitor started successfully");

        // Spawn a task to log sidecar output
        tauri::async_runtime::spawn(async move {
            use tauri_plugin_shell::process::CommandEvent;

            while let Some(event) = rx.recv().await {
                match event {
                    CommandEvent::Stdout(line) => {
                        if let Ok(text) = String::from_utf8(line) {
                            let trimmed = text.trim();
                            if !trimmed.is_empty() {
                                log::debug!("Sidecar stdout: {}", trimmed);
                                println!("[TextMonitor] {}", trimmed);
                            }
                        }
                    }
                    CommandEvent::Stderr(line) => {
                        if let Ok(text) = String::from_utf8(line) {
                            let trimmed = text.trim();
                            if !trimmed.is_empty() {
                                log::warn!("Sidecar stderr: {}", trimmed);
                                eprintln!("[TextMonitor Error] {}", trimmed);
                            }
                        }
                    }
                    CommandEvent::Error(err) => {
                        log::error!("Sidecar error: {}", err);
                        eprintln!("[TextMonitor Error] {}", err);
                    }
                    CommandEvent::Terminated(payload) => {
                        log::info!(
                            "Sidecar terminated with code: {:?}, signal: {:?}",
                            payload.code,
                            payload.signal
                        );
                        println!("[Sidecar] Terminated with code: {:?}", payload.code);
                        break;
                    }
                    _ => {}
                }
            }
        });

        Ok(())
    }

    /// Stops the text-monitor sidecar if running.
    pub async fn stop(&self) -> Result<(), String> {
        let mut guard = self.child.lock().await;

        if let Some(child) = guard.take() {
            log::info!("Sidecar: Stopping text-monitor...");

            child
                .kill()
                .map_err(|e| format!("Failed to kill sidecar: {}", e))?;

            log::info!("Sidecar: Text monitor stopped");
        } else {
            log::debug!("Sidecar: Text monitor was not running");
        }

        Ok(())
    }

    /// Checks if the sidecar is currently running.
    pub async fn is_running(&self) -> bool {
        self.child.lock().await.is_some()
    }
}

impl Default for SidecarState {
    fn default() -> Self {
        Self::new()
    }
}

/// Tauri command to manually start the text monitor sidecar.
#[tauri::command]
pub async fn start_text_monitor(
    app: AppHandle,
    state: tauri::State<'_, SidecarState>,
) -> Result<(), String> {
    state.start(&app).await
}

/// Tauri command to manually stop the text monitor sidecar.
#[tauri::command]
pub async fn stop_text_monitor(
    state: tauri::State<'_, SidecarState>,
) -> Result<(), String> {
    state.stop().await
}

/// Tauri command to check if the text monitor is running.
#[tauri::command]
pub async fn is_text_monitor_running(
    state: tauri::State<'_, SidecarState>,
) -> Result<bool, String> {
    Ok(state.is_running().await)
}
