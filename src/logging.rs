use anyhow::{Context, Result};
use crossbeam_channel::{unbounded, Sender};
use std::fs::{OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

use crate::tui::ansi::strip_ansi;

#[allow(dead_code)]
pub enum LoggerCommand {
    Log { timestamp: String, message: String },
    Flush,
    Stop,
}

pub struct FileLogger {
    path: PathBuf,
    sender: Sender<LoggerCommand>,
    is_active: Arc<AtomicBool>,
    thread_handle: Option<JoinHandle<()>>,
}

impl FileLogger {
    pub fn start<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path_buf = path.as_ref().to_path_buf();
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path_buf)
            .with_context(|| format!("Failed to open log file at {:?}", path_buf))?;

        let (sender, receiver) = unbounded::<LoggerCommand>();
        let is_active = Arc::new(AtomicBool::new(true));
        let is_active_clone = is_active.clone();

        let thread_handle = thread::spawn(move || {
            let mut writer = BufWriter::new(file);
            while let Ok(cmd) = receiver.recv() {
                match cmd {
                    LoggerCommand::Log { timestamp, message } => {
                        let clean_msg = strip_ansi(&message);
                        let line = if timestamp.is_empty() {
                            format!("{}\n", clean_msg)
                        } else {
                            format!("[{}] {}\n", timestamp, clean_msg)
                        };
                        let _ = writer.write_all(line.as_bytes());
                        let _ = writer.flush();
                    }
                    LoggerCommand::Flush => {
                        let _ = writer.flush();
                    }
                    LoggerCommand::Stop => {
                        let _ = writer.flush();
                        break;
                    }
                }
            }
            let _ = writer.flush();
            is_active_clone.store(false, Ordering::SeqCst);
        });

        Ok(Self {
            path: path_buf,
            sender,
            is_active,
            thread_handle: Some(thread_handle),
        })
    }

    pub fn log(&self, timestamp: &str, message: &str) {
        if self.is_active() {
            let _ = self.sender.send(LoggerCommand::Log {
                timestamp: timestamp.to_string(),
                message: message.to_string(),
            });
        }
    }

    #[allow(dead_code)]
    pub fn flush(&self) {
        if self.is_active() {
            let _ = self.sender.send(LoggerCommand::Flush);
        }
    }

    pub fn is_active(&self) -> bool {
        self.is_active.load(Ordering::SeqCst)
    }

    #[allow(dead_code)]
    pub fn path(&self) -> &Path {
        &self.path
    }

    #[allow(dead_code)]
    pub fn path_str(&self) -> String {
        self.path.to_string_lossy().into_owned()
    }

    pub fn stop(&mut self) {
        if let Some(handle) = self.thread_handle.take() {
            let _ = self.sender.send(LoggerCommand::Stop);
            let _ = handle.join();
        }
        self.is_active.store(false, Ordering::SeqCst);
    }
}

impl Drop for FileLogger {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_file_logger_writes_clean_log() -> Result<()> {
        let temp_dir = std::env::temp_dir();
        let log_file = temp_dir.join("rtt_test_log_stage4.log");
        if log_file.exists() {
            let _ = fs::remove_file(&log_file);
        }

        let mut logger = FileLogger::start(&log_file)?;
        assert!(logger.is_active());
        assert_eq!(logger.path(), log_file.as_path());

        logger.log("14:30:15.123", "\x1b[32m<info>\x1b[0m System started");
        logger.log("14:30:15.456", "\x1b[31m<error>\x1b[0m Disk failure\r\n");
        logger.stop();

        assert!(!logger.is_active());

        let contents = fs::read_to_string(&log_file)?;
        assert!(contents.contains("[14:30:15.123] <info> System started"));
        assert!(contents.contains("[14:30:15.456] <error> Disk failure"));
        assert!(!contents.contains("\x1b["));

        let _ = fs::remove_file(&log_file);
        Ok(())
    }
}
