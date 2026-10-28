//! 极简文件日志：只记错误与关键事件，不记录用户查询的词条内容。
//! 单文件 1MB 滚动保留一份，避免偷偷长大。

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use log::{Log, Metadata, Record};

const MAX_BYTES: u64 = 1024 * 1024;

struct FileLogger {
    path: PathBuf,
    lock: Mutex<()>,
}

impl Log for FileLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= log::Level::Info
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let Ok(_guard) = self.lock.lock() else {
            return;
        };
        if let Ok(meta) = fs::metadata(&self.path) {
            if meta.len() > MAX_BYTES {
                let _ = fs::rename(&self.path, self.path.with_extension("log.1"));
            }
        }
        if let Ok(mut file) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
        {
            let _ = writeln!(
                file,
                "[{}] {:<5} {}",
                stamp(),
                record.level(),
                record.args()
            );
        }
    }

    fn flush(&self) {}
}

fn stamp() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{now}")
}

pub fn init(base_dir: &Path) {
    let dir = base_dir.join("logs");
    if fs::create_dir_all(&dir).is_err() {
        return;
    }
    let logger = FileLogger {
        path: dir.join("app.log"),
        lock: Mutex::new(()),
    };
    if log::set_boxed_logger(Box::new(logger)).is_ok() {
        log::set_max_level(log::LevelFilter::Info);
    }
}
