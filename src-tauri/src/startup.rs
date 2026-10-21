use serde_json::{json, Value};
use std::{
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
    time::Instant,
};

pub struct Startup {
    started: Instant,
    reported: AtomicBool,
    report_path: Option<PathBuf>,
}

impl Default for Startup {
    fn default() -> Self {
        Self::new()
    }
}

impl Startup {
    pub fn new() -> Self {
        Self {
            started: Instant::now(),
            reported: AtomicBool::new(false),
            report_path: std::env::var_os("SNOR_STARTUP_REPORT").map(PathBuf::from),
        }
    }

    pub fn ready(&self) -> Value {
        let result = json!({"backendToFrontendReadyMs":self.started.elapsed().as_secs_f64()*1000.0,"processId":std::process::id(),"profile":if cfg!(debug_assertions){"development"}else{"release"},"version":env!("CARGO_PKG_VERSION"),"note":"Elapsed from Rust entry to frontend DOM-ready IPC; not first token or full visual paint."});
        if !self.reported.swap(true, Ordering::AcqRel) {
            if let Some(path) = &self.report_path {
                let temporary = path.with_extension("tmp");
                if std::fs::write(&temporary, result.to_string()).is_ok() {
                    let _ = std::fs::rename(temporary, path);
                }
            }
        }
        result
    }
}
