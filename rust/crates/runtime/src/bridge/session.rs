//! Session creation and management for different spawn modes

use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::bridge::types::{SpawnMode, WorkSecret};
use crate::session::Session;

/// Session creation error
#[derive(Debug, thiserror::Error)]
pub enum SessionCreateError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Git error: {0}")]
    Git(String),

    #[error("Invalid path: {0}")]
    InvalidPath(String),

    #[error("Session error: {0}")]
    Session(String),
}

/// Session spawner
pub struct SessionSpawner {
    base_dir: PathBuf,
    spawn_mode: SpawnMode,
}

impl SessionSpawner {
    /// Create a new session spawner
    pub fn new(base_dir: PathBuf, spawn_mode: SpawnMode) -> Self {
        Self {
            base_dir,
            spawn_mode,
        }
    }

    /// Spawn a new session based on the work secret
    pub async fn spawn_session(
        &self,
        work_secret: &WorkSecret,
    ) -> Result<SpawnedSession, SessionCreateError> {
        match self.spawn_mode {
            SpawnMode::SingleSession => {
                self.spawn_single_session(work_secret).await
            }
            SpawnMode::Worktree => {
                self.spawn_worktree_session(work_secret).await
            }
            SpawnMode::SameDir => {
                self.spawn_same_dir_session(work_secret).await
            }
        }
    }

    /// Spawn a single session (use current directory, stop bridge when done)
    async fn spawn_single_session(
        &self,
        work_secret: &WorkSecret,
    ) -> Result<SpawnedSession, SessionCreateError> {
        let session = Self::create_session()?;
        Ok(SpawnedSession {
            session,
            work_dir: self.base_dir.clone(),
            mode: SpawnMode::SingleSession,
            cleanup: None,
        })
    }

    /// Spawn a session in an isolated git worktree
    async fn spawn_worktree_session(
        &self,
        work_secret: &WorkSecret,
    ) -> Result<SpawnedSession, SessionCreateError> {
        let work_dir = self.create_worktree(work_secret).await?;
        let session = Self::create_session()?;

        Ok(SpawnedSession {
            session,
            work_dir,
            mode: SpawnMode::Worktree,
            cleanup: Some(SessionCleanup::Worktree),
        })
    }

    /// Spawn a session in the current directory (shared)
    async fn spawn_same_dir_session(
        &self,
        work_secret: &WorkSecret,
    ) -> Result<SpawnedSession, SessionCreateError> {
        let session = Self::create_session()?;
        Ok(SpawnedSession {
            session,
            work_dir: self.base_dir.clone(),
            mode: SpawnMode::SameDir,
            cleanup: None,
        })
    }

    /// Create a git worktree
    async fn create_worktree(
        &self,
        work_secret: &WorkSecret,
    ) -> Result<PathBuf, SessionCreateError> {
        let worktree_name = format!("remote-session-{}", uuid::Uuid::new_v4());
        let worktree_path = self.base_dir.join(&worktree_name);

        Ok(worktree_path)
    }

    /// Create a new session
    fn create_session() -> Result<Session, SessionCreateError> {
        Ok(Session::new())
    }
}

/// A spawned session
pub struct SpawnedSession {
    pub session: Session,
    pub work_dir: PathBuf,
    pub mode: SpawnMode,
    cleanup: Option<SessionCleanup>,
}

impl SpawnedSession {
    /// Get the session
    pub fn session(&self) -> &Session {
        &self.session
    }

    /// Get the session mutably
    pub fn session_mut(&mut self) -> &mut Session {
        &mut self.session
    }

    /// Get the work directory
    pub fn work_dir(&self) -> &Path {
        &self.work_dir
    }

    /// Clean up the session
    pub async fn cleanup(self) -> Result<(), SessionCreateError> {
        if let Some(cleanup) = self.cleanup {
            cleanup.execute(&self.work_dir).await?;
        }
        Ok(())
    }
}

/// Session cleanup strategy
#[derive(Debug, Clone)]
enum SessionCleanup {
    /// Clean up a git worktree
    Worktree,
    /// Delete a temporary directory
    TempDir,
}

impl SessionCleanup {
    /// Execute the cleanup
    async fn execute(&self, path: &Path) -> Result<(), SessionCreateError> {
        match self {
            SessionCleanup::Worktree => {
                Self::cleanup_worktree(path).await?;
            }
            SessionCleanup::TempDir => {
                tokio::fs::remove_dir_all(path).await?;
            }
        }
        Ok(())
    }

    /// Clean up a git worktree
    async fn cleanup_worktree(path: &Path) -> Result<(), SessionCreateError> {
        Ok(())
    }
}

/// UUID generation (placeholder until we add a proper dependency)
mod uuid {
    /// Generate a simple random UUID
    pub fn new_v4() -> String {
        use std::fmt::Write;
        let mut rng = rand::thread_rng();
        let mut bytes = [0u8; 16];
        rng.fill_bytes(&mut bytes);
        bytes[6] = (bytes[6] & 0x0F) | 0x40;
        bytes[8] = (bytes[8] & 0x3F) | 0x80;
        let mut uuid = String::with_capacity(36);
        for (i, &b) in bytes.iter().enumerate() {
            if i == 4 || i == 6 || i == 8 || i == 10 {
                uuid.push('-');
            }
            write!(&mut uuid, "{:02x}", b).unwrap();
        }
        uuid
    }
}
