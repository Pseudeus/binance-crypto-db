use thiserror::Error;

pub mod backup_actor;

#[derive(Error, Debug)]
pub enum BackupScriptError {
    #[error("Usage Error (Code 1): Incorrect arguments passed to script")]
    UsageError,
    #[error("Dependency Missing (Code 2): Host is missing sqlite3, zstd, or rclone")]
    DependencyMissing,
    #[error("Directory Not Found (Code 3): Working directory invalid")]
    DirNotFound,
    #[error("File Not Found (Code 4): Database file missing")]
    FileNotFound,
    #[error("Dump Failed (Code 5): sqlite3 .dump command failed")]
    DumpFailed,
    #[error("Compression Failed (Code 6): zstd compression failed")]
    CompressFailed,
    #[error("Upload Failed (Code 7): rclone copy failed")]
    UploadFailed,
    #[error("Move Failed (Code 8): Failed to archive original DB")]
    MoveFailed,
    #[error("Unknown Script Error (Code {0}): The script crashed with an unhandled exit code")]
    Unknown(i32),
}

impl From<i32> for BackupScriptError {
    fn from(value: i32) -> Self {
        match value {
            1 => Self::UsageError,
            2 => Self::DependencyMissing,
            3 => Self::DirNotFound,
            4 => Self::FileNotFound,
            5 => Self::DumpFailed,
            6 => Self::CompressFailed,
            7 => Self::UploadFailed,
            8 => Self::MoveFailed,
            c => Self::Unknown(c),
        }
    }
}
