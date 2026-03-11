//! Sidecar management for the text monitor service.
//!
//! On Windows: manages the .NET Text Monitor sidecar process.
//! On macOS: delegates to the native in-process monitor (macos_monitor module).

use std::sync::Arc;
use tauri::async_runtime::Mutex;
use tauri::AppHandle;

#[cfg(target_os = "windows")]
use tauri_plugin_shell::process::CommandChild;
#[cfg(target_os = "windows")]
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
#[cfg(target_os = "windows")]
fn kill_existing_text_monitor() {
    use std::process::Command;
    use std::os::windows::process::CommandExt;

    let names = [
        "text-monitor-x86_64-pc-windows-msvc.exe",
        "text-monitor.exe",
    ];

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

/// State for managing the text monitor (sidecar on Windows, native on macOS).
pub struct SidecarState {
    #[cfg(target_os = "windows")]
    child: Arc<Mutex<Option<CommandChild>>>,
    #[cfg(not(target_os = "windows"))]
    _running: Arc<Mutex<bool>>,
}

impl SidecarState {
    pub fn new() -> Self {
        Self {
            #[cfg(target_os = "windows")]
            child: Arc::new(Mutex::new(None)),
            #[cfg(not(target_os = "windows"))]
            _running: Arc::new(Mutex::new(false)),
        }
    }

    /// Starts the text monitor.
    /// On Windows: spawns the .NET sidecar process.
    /// On macOS: no-op (native monitor is started separately in lib.rs).
    #[cfg(target_os = "windows")]
    pub async fn start(&self, app: &AppHandle) -> Result<(), String> {
        let mut guard = self.child.lock().await;

        if guard.is_some() {
            log::info!("Sidecar: Text monitor already running");
            return Ok(());
        }

        kill_existing_text_monitor();

        log::info!("Sidecar: Starting text-monitor...");

        let sidecar_command = match app.shell().sidecar("text-monitor") {
            Ok(cmd) => cmd,
            Err(e) => {
                let msg = format!("Failed to create sidecar command: {}", e);
                log::error!("Sidecar: {}", msg);
                return Err(msg);
            }
        };

        let (mut rx, child) = match sidecar_command.spawn() {
            Ok(result) => result,
            Err(e) => {
                let msg = format!("Failed to spawn sidecar: {}", e);
                log::error!("Sidecar: {}", msg);
                return Err(msg);
            }
        };

        *guard = Some(child);
        log::info!("Sidecar: Text monitor started successfully");

        tauri::async_runtime::spawn(async move {
            use tauri_plugin_shell::process::CommandEvent;

            while let Some(event) = rx.recv().await {
                match event {
                    CommandEvent::Stdout(line) => {
                        if let Ok(text) = String::from_utf8(line) {
                            let trimmed = text.trim();
                            if !trimmed.is_empty() {
                                log::debug!("Sidecar stdout: {}", trimmed);
                            }
                        }
                    }
                    CommandEvent::Stderr(line) => {
                        if let Ok(text) = String::from_utf8(line) {
                            let trimmed = text.trim();
                            if !trimmed.is_empty() {
                                log::warn!("Sidecar stderr: {}", trimmed);
                            }
                        }
                    }
                    CommandEvent::Error(err) => {
                        log::error!("Sidecar error: {}", err);
                    }
                    CommandEvent::Terminated(payload) => {
                        log::info!(
                            "Sidecar terminated with code: {:?}, signal: {:?}",
                            payload.code,
                            payload.signal
                        );
                        break;
                    }
                    _ => {}
                }
            }
        });

        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    pub async fn start(&self, _app: &AppHandle) -> Result<(), String> {
        log::info!("Sidecar: No sidecar needed on this platform");
        Ok(())
    }

    /// Stops the text monitor.
    #[cfg(target_os = "windows")]
    pub async fn stop(&self) -> Result<(), String> {
        let mut guard = self.child.lock().await;

        if let Some(child) = guard.take() {
            log::info!("Sidecar: Stopping text-monitor...");
            child
                .kill()
                .map_err(|e| format!("Failed to kill sidecar: {}", e))?;
            log::info!("Sidecar: Text monitor stopped");
        }

        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    pub async fn stop(&self) -> Result<(), String> {
        Ok(())
    }

    /// Checks if the text monitor is currently running.
    #[cfg(target_os = "windows")]
    pub async fn is_running(&self) -> bool {
        self.child.lock().await.is_some()
    }

    #[cfg(not(target_os = "windows"))]
    pub async fn is_running(&self) -> bool {
        false
    }
}

impl Default for SidecarState {
    fn default() -> Self {
        Self::new()
    }
}

/// Tauri command to manually start the text monitor.
#[tauri::command]
pub async fn start_text_monitor(
    app: AppHandle,
    state: tauri::State<'_, SidecarState>,
) -> Result<(), String> {
    state.start(&app).await
}

/// Tauri command to manually stop the text monitor.
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
