//! Owns the Windows job containing a Claude process and its descendants.
//! Closing the job handle terminates descendants, including on app exit.
#[cfg(windows)]
pub struct ProcessGuard(windows_sys::Win32::Foundation::HANDLE);

#[cfg(windows)]
unsafe impl Send for ProcessGuard {}

#[cfg(windows)]
impl ProcessGuard {
    pub fn attach(child: &tokio::process::Child) -> Result<Self, String> {
        use windows_sys::Win32::{Foundation::CloseHandle, System::JobObjects::*};
        let process = child
            .raw_handle()
            .ok_or("Handle del processo non disponibile")?;
        // SAFETY: The handle returned by CreateJobObjectW is owned here. The information
        // pointer lives for the duration of the call and has exactly the documented size.
        // The child process handle is borrowed and is never closed by this guard.
        unsafe {
            let job = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            if job.is_null() {
                return Err(format!(
                    "Creazione del job Windows fallita: {}",
                    std::io::Error::last_os_error()
                ));
            }
            let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            if SetInformationJobObject(
                job,
                JobObjectExtendedLimitInformation,
                &limits as *const _ as *const _,
                std::mem::size_of_val(&limits) as u32,
            ) == 0
                || AssignProcessToJobObject(job, process) == 0
            {
                let error = std::io::Error::last_os_error();
                CloseHandle(job);
                return Err(format!(
                    "Impossibile isolare il ciclo di vita dei processi Claude: {error}"
                ));
            }
            Ok(Self(job))
        }
    }
}

#[cfg(windows)]
impl Drop for ProcessGuard {
    fn drop(&mut self) {
        // SAFETY: This is the unique owning handle and is closed exactly once.
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(self.0);
        }
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use std::process::Stdio;
    use tokio::io::{AsyncBufReadExt, BufReader};

    #[tokio::test]
    async fn closing_the_job_terminates_the_process_and_its_descendant() {
        let script = r#"const {spawn}=require('node:child_process');const c=spawn(process.execPath,['-e','setInterval(()=>{},1000)'],{stdio:'ignore'});console.log(c.pid);setInterval(()=>{},1000);"#;
        let mut child = crate::agent::command("node")
            .args(["-e", script])
            .stdout(Stdio::piped())
            .spawn()
            .expect("Node is required for the Windows process lifecycle test");
        let guard = ProcessGuard::attach(&child).unwrap();
        let mut line = String::new();
        let mut stdout = BufReader::new(child.stdout.take().unwrap());
        tokio::time::timeout(
            std::time::Duration::from_secs(10),
            stdout.read_line(&mut line),
        )
        .await
        .unwrap()
        .unwrap();
        let descendant: u32 = line.trim().parse().unwrap();
        // SAFETY: The returned handle is owned by this test and closed below.
        // It requests synchronization only and never modifies the process.
        let handle = unsafe {
            windows_sys::Win32::System::Threading::OpenProcess(0x00100000, 0, descendant)
        };
        assert!(!handle.is_null());
        drop(guard);
        tokio::time::timeout(std::time::Duration::from_secs(5), child.wait())
            .await
            .unwrap()
            .unwrap();
        // SAFETY: The process handle remains valid until CloseHandle below.
        let wait =
            unsafe { windows_sys::Win32::System::Threading::WaitForSingleObject(handle, 3000) };
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(handle);
        }
        assert_eq!(wait, windows_sys::Win32::Foundation::WAIT_OBJECT_0);
    }
}

#[cfg(not(windows))]
pub struct ProcessGuard;
#[cfg(not(windows))]
impl ProcessGuard {
    pub fn attach(_: &tokio::process::Child) -> Result<Self, String> {
        Ok(Self)
    }
}
