use anyhow::{anyhow, Result};
use libloading::Library;
use std::env;
use std::ffi::{c_char, c_void, CStr, CString};
use std::path::{Path, PathBuf};
use std::ptr;

#[allow(dead_code)]
pub const JLINK_TIF_JTAG: i32 = 0;
pub const JLINK_TIF_SWD: i32 = 1;

pub const JLINK_RTT_CMD_START: u32 = 0;
pub const JLINK_RTT_CMD_STOP: u32 = 1;
#[allow(dead_code)]
pub const JLINK_RTT_CMD_GETNUMBUFS: u32 = 2;
#[allow(dead_code)]
pub const JLINK_RTT_CMD_GETSTAT: u32 = 3;

/// Finds the JLink_x64.dll library path on the system.
pub fn find_jlink_dll() -> Option<PathBuf> {
    // 1. Environment variable override
    if let Ok(custom_path) = env::var("JLINK_DLL_PATH") {
        let p = PathBuf::from(custom_path);
        if p.exists() {
            return Some(p);
        }
    }

    // 2. Scan standard 64-bit Program Files directory
    #[cfg(target_os = "windows")]
    {
        let prog_files = env::var("ProgramFiles").unwrap_or_else(|_| r"C:\Program Files".to_string());
        let segger_dir = Path::new(&prog_files).join("SEGGER");
        if segger_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&segger_dir) {
                let mut candidates: Vec<PathBuf> = Vec::new();
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        let name = path.file_name().unwrap_or_default().to_string_lossy();
                        if name.starts_with("JLink") {
                            let dll = path.join("JLink_x64.dll");
                            if dll.exists() {
                                candidates.push(dll);
                            }
                        }
                    }
                }
                candidates.sort_by(|a, b| b.cmp(a)); // Newest first
                if let Some(newest) = candidates.into_iter().next() {
                    return Some(newest);
                }
            }
        }
    }

    // 3. Fallback to generic library resolution
    Some(PathBuf::from("JLink_x64.dll"))
}

pub struct SeggerJLink {
    _lib: Library,
    pub dll_path: PathBuf,
    fn_close: unsafe extern "C" fn(),
    fn_is_open: unsafe extern "C" fn() -> i32,
    fn_exec_command: unsafe extern "C" fn(*const c_char, *mut c_char, i32) -> i32,
    fn_tif_select: unsafe extern "C" fn(i32) -> i32,
    fn_set_speed: unsafe extern "C" fn(u32),
    fn_connect: unsafe extern "C" fn() -> i32,
    fn_rtt_control: unsafe extern "C" fn(u32, *mut c_void) -> i32,
    fn_rtt_read: unsafe extern "C" fn(u32, *mut u8, u32) -> i32,
    fn_rtt_write: unsafe extern "C" fn(u32, *const u8, u32) -> i32,
    fn_reset: unsafe extern "C" fn(),
    fn_halt: unsafe extern "C" fn() -> i32,
    fn_go: unsafe extern "C" fn(),
    fn_get_sn: unsafe extern "C" fn() -> u32,
}

// Safety: The Segger DLL handles its own synchronization and can be safely sent to a worker thread.
unsafe impl Send for SeggerJLink {}

impl SeggerJLink {
    /// Loads the Segger J-Link dynamic library and connects to the target device.
    pub fn open(device: &str, speed_khz: u32) -> Result<Self> {
        let dll_path = find_jlink_dll()
            .ok_or_else(|| anyhow!("Could not find JLink_x64.dll. Please install Segger J-Link Software Pack."))?;

        let lib = unsafe { Library::new(&dll_path) }
            .map_err(|e| anyhow!("Failed to load JLink DLL at {:?}: {}", dll_path, e))?;

        unsafe {
            // Load exported functions (trying JLINKARM_* or JLINK_* prefix)
            let fn_open: unsafe extern "C" fn() -> *const c_char = *lib
                .get(b"JLINKARM_Open\0")
                .or_else(|_| lib.get(b"JLINK_Open\0"))
                .map_err(|e| anyhow!("Failed to locate JLINK_Open symbol: {}", e))?;

            let fn_close: unsafe extern "C" fn() = *lib
                .get(b"JLINKARM_Close\0")
                .or_else(|_| lib.get(b"JLINK_Close\0"))
                .map_err(|e| anyhow!("Failed to locate JLINK_Close symbol: {}", e))?;

            let fn_is_open: unsafe extern "C" fn() -> i32 = *lib
                .get(b"JLINKARM_IsOpen\0")
                .or_else(|_| lib.get(b"JLINK_IsOpen\0"))
                .map_err(|e| anyhow!("Failed to locate JLINK_IsOpen symbol: {}", e))?;

            let fn_exec_command: unsafe extern "C" fn(*const c_char, *mut c_char, i32) -> i32 = *lib
                .get(b"JLINKARM_ExecCommand\0")
                .or_else(|_| lib.get(b"JLINK_ExecCommand\0"))
                .map_err(|e| anyhow!("Failed to locate JLINK_ExecCommand symbol: {}", e))?;

            let fn_tif_select: unsafe extern "C" fn(i32) -> i32 = *lib
                .get(b"JLINKARM_TIF_Select\0")
                .or_else(|_| lib.get(b"JLINK_TIF_Select\0"))
                .map_err(|e| anyhow!("Failed to locate JLINK_TIF_Select symbol: {}", e))?;

            let fn_set_speed: unsafe extern "C" fn(u32) = *lib
                .get(b"JLINKARM_SetSpeed\0")
                .or_else(|_| lib.get(b"JLINK_SetSpeed\0"))
                .map_err(|e| anyhow!("Failed to locate JLINK_SetSpeed symbol: {}", e))?;

            let fn_connect: unsafe extern "C" fn() -> i32 = *lib
                .get(b"JLINKARM_Connect\0")
                .or_else(|_| lib.get(b"JLINK_Connect\0"))
                .map_err(|e| anyhow!("Failed to locate JLINK_Connect symbol: {}", e))?;

            let fn_rtt_control: unsafe extern "C" fn(u32, *mut c_void) -> i32 = *lib
                .get(b"JLINKARM_RTTERMINAL_Control\0")
                .or_else(|_| lib.get(b"JLINK_RTTERMINAL_Control\0"))
                .map_err(|e| anyhow!("Failed to locate JLINK_RTTERMINAL_Control symbol: {}", e))?;

            let fn_rtt_read: unsafe extern "C" fn(u32, *mut u8, u32) -> i32 = *lib
                .get(b"JLINKARM_RTTERMINAL_Read\0")
                .or_else(|_| lib.get(b"JLINK_RTTERMINAL_Read\0"))
                .map_err(|e| anyhow!("Failed to locate JLINK_RTTERMINAL_Read symbol: {}", e))?;

            let fn_rtt_write: unsafe extern "C" fn(u32, *const u8, u32) -> i32 = *lib
                .get(b"JLINKARM_RTTERMINAL_Write\0")
                .or_else(|_| lib.get(b"JLINK_RTTERMINAL_Write\0"))
                .map_err(|e| anyhow!("Failed to locate JLINK_RTTERMINAL_Write symbol: {}", e))?;

            let fn_reset: unsafe extern "C" fn() = *lib
                .get(b"JLINKARM_Reset\0")
                .or_else(|_| lib.get(b"JLINK_Reset\0"))
                .map_err(|e| anyhow!("Failed to locate JLINK_Reset symbol: {}", e))?;

            let fn_halt: unsafe extern "C" fn() -> i32 = *lib
                .get(b"JLINKARM_Halt\0")
                .or_else(|_| lib.get(b"JLINK_Halt\0"))
                .map_err(|e| anyhow!("Failed to locate JLINK_Halt symbol: {}", e))?;

            let fn_go: unsafe extern "C" fn() = *lib
                .get(b"JLINKARM_Go\0")
                .or_else(|_| lib.get(b"JLINK_Go\0"))
                .map_err(|e| anyhow!("Failed to locate JLINK_Go symbol: {}", e))?;

            let fn_get_sn: unsafe extern "C" fn() -> u32 = *lib
                .get(b"JLINKARM_GetSN\0")
                .or_else(|_| lib.get(b"JLINK_GetSN\0"))
                .map_err(|e| anyhow!("Failed to locate JLINK_GetSN symbol: {}", e))?;

            // 1. Open the J-Link connection (starts multiplexer if another app is active)
            let err_ptr = fn_open();
            if !err_ptr.is_null() {
                let err_str = CStr::from_ptr(err_ptr).to_string_lossy();
                if !err_str.is_empty() {
                    return Err(anyhow!("JLINK_Open error: {}", err_str));
                }
            }

            let segger = Self {
                _lib: lib,
                dll_path,
                fn_close,
                fn_is_open,
                fn_exec_command,
                fn_tif_select,
                fn_set_speed,
                fn_connect,
                fn_rtt_control,
                fn_rtt_read,
                fn_rtt_write,
                fn_reset,
                fn_halt,
                fn_go,
                fn_get_sn,
            };

            // 2. Select SWD interface
            let tif_res = (segger.fn_tif_select)(JLINK_TIF_SWD);
            if tif_res < 0 {
                return Err(anyhow!("Failed to select SWD interface (error code: {})", tif_res));
            }

            // 3. Configure target device
            let dev_cmd = format!("device = {}", device);
            let cmd_c = CString::new(dev_cmd)?;
            (segger.fn_exec_command)(cmd_c.as_ptr(), ptr::null_mut(), 0);

            // 4. Configure clock speed
            if speed_khz > 0 {
                (segger.fn_set_speed)(speed_khz);
            }

            // 5. Connect to target core
            let conn_res = (segger.fn_connect)();
            if conn_res < 0 {
                return Err(anyhow!(
                    "JLINK_Connect failed with code {}. Please check if the microcontroller is powered.",
                    conn_res
                ));
            }

            Ok(segger)
        }
    }

    /// Gets the serial number of the connected J-Link probe.
    pub fn get_serial_number(&self) -> u32 {
        unsafe { (self.fn_get_sn)() }
    }

    /// Executes a custom command string in the J-Link engine.
    #[allow(dead_code)]
    pub fn exec_command(&self, cmd: &str) -> Result<String> {
        let mut out_buf = vec![0u8; 1024];
        let cmd_c = CString::new(cmd)?;
        let res = unsafe {
            (self.fn_exec_command)(cmd_c.as_ptr(), out_buf.as_mut_ptr() as *mut c_char, out_buf.len() as i32)
        };
        if res < 0 {
            return Err(anyhow!("JLINK_ExecCommand failed with code {}", res));
        }
        let out_str = unsafe { CStr::from_ptr(out_buf.as_ptr() as *const c_char) };
        Ok(out_str.to_string_lossy().into_owned())
    }

    /// Starts the Segger RTT terminal engine.
    pub fn start_rtt(&self) -> Result<()> {
        let res = unsafe { (self.fn_rtt_control)(JLINK_RTT_CMD_START, ptr::null_mut()) };
        if res < 0 {
            return Err(anyhow!("Failed to start Segger RTT engine (code: {})", res));
        }
        Ok(())
    }

    /// Stops the Segger RTT terminal engine.
    pub fn stop_rtt(&self) {
        unsafe {
            (self.fn_rtt_control)(JLINK_RTT_CMD_STOP, ptr::null_mut());
        }
    }

    /// Reads incoming bytes from the specified RTT Up channel.
    pub fn read_rtt(&self, channel: u32, buf: &mut [u8]) -> Result<usize> {
        let read = unsafe {
            (self.fn_rtt_read)(channel, buf.as_mut_ptr(), buf.len() as u32)
        };
        if read < 0 {
            Err(anyhow!("Error reading RTT channel {} (code: {})", channel, read))
        } else {
            Ok(read as usize)
        }
    }

    /// Writes outgoing bytes to the specified RTT Down channel.
    pub fn write_rtt(&self, channel: u32, data: &[u8]) -> Result<usize> {
        let written = unsafe {
            (self.fn_rtt_write)(channel, data.as_ptr(), data.len() as u32)
        };
        if written < 0 {
            Err(anyhow!("Error writing RTT channel {} (code: {})", channel, written))
        } else {
            Ok(written as usize)
        }
    }

    /// Triggers a hardware reset of the target device.
    pub fn reset_target(&self) {
        unsafe {
            (self.fn_reset)();
        }
    }

    /// Halts the target CPU.
    pub fn halt_target(&self) -> Result<()> {
        let res = unsafe { (self.fn_halt)() };
        if res < 0 {
            Err(anyhow!("Failed to halt target (code: {})", res))
        } else {
            Ok(())
        }
    }

    /// Resumes execution of the target CPU.
    pub fn resume_target(&self) {
        unsafe {
            (self.fn_go)();
        }
    }
}

impl Drop for SeggerJLink {
    fn drop(&mut self) {
        self.stop_rtt();
        unsafe {
            if (self.fn_is_open)() != 0 {
                (self.fn_close)();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_segger_dll() {
        let dll = find_jlink_dll();
        assert!(dll.is_some(), "Should locate JLink DLL or default on system");
    }
}
