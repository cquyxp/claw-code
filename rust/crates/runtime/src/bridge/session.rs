//! Session creation and management for different spawn modes

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::bridge::types::{SpawnMode, WorkSecret, GitInfo, SourceConfig, AuthConfig};
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

    #[error("Command execution error: {0}")]
    Command(String),
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
        let work_dir = self.base_dir.clone();

        // Setup environment variables
        Self::setup_environment(work_secret)?;

        // Setup git config
        Self::setup_git_config(&work_dir, work_secret).await?;

        let session = Self::create_session()?;
        Ok(SpawnedSession {
            session,
            work_dir,
            mode: SpawnMode::SingleSession,
            cleanup: None,
            env_vars: work_secret.environment_variables.clone().unwrap_or_default(),
        })
    }

    /// Spawn a session in an isolated git worktree
    async fn spawn_worktree_session(
        &self,
        work_secret: &WorkSecret,
    ) -> Result<SpawnedSession, SessionCreateError> {
        let work_dir = self.create_worktree(work_secret).await?;

        // Setup environment variables
        Self::setup_environment(work_secret)?;

        // Setup git config in the worktree
        Self::setup_git_config(&work_dir, work_secret).await?;

        let session = Self::create_session()?;

        Ok(SpawnedSession {
            session,
            work_dir,
            mode: SpawnMode::Worktree,
            cleanup: Some(SessionCleanup::Worktree),
            env_vars: work_secret.environment_variables.clone().unwrap_or_default(),
        })
    }

    /// Spawn a session in the current directory (shared)
    async fn spawn_same_dir_session(
        &self,
        work_secret: &WorkSecret,
    ) -> Result<SpawnedSession, SessionCreateError> {
        let work_dir = self.base_dir.clone();

        // Setup environment variables
        Self::setup_environment(work_secret)?;

        // Setup git config
        Self::setup_git_config(&work_dir, work_secret).await?;

        let session = Self::create_session()?;
        Ok(SpawnedSession {
            session,
            work_dir,
            mode: SpawnMode::SameDir,
            cleanup: None,
            env_vars: work_secret.environment_variables.clone().unwrap_or_default(),
        })
    }

    /// Create a git worktree
    async fn create_worktree(
        &self,
        work_secret: &WorkSecret,
    ) -> Result<PathBuf, SessionCreateError> {
        // Check if we have git info
        let git_info = work_secret.sources.iter().find_map(|s| s.git_info.as_ref());

        let worktree_name = format!("remote-session-{}", generate_uuid());
        let worktree_path = std::env::temp_dir().join(&worktree_name);

        // Create the directory
        tokio::fs::create_dir_all(&worktree_path).await?;

        // If we have git info, clone/checkout the repo
        if let Some(git) = git_info {
            Self::setup_git_repository(&worktree_path, git).await?;
        }

        Ok(worktree_path)
    }

    /// Setup a git repository
    async fn setup_git_repository(
        path: &Path,
        git_info: &GitInfo,
    ) -> Result<(), SessionCreateError> {
        // Clone the repo if we have a URL
        if !git_info.repo.is_empty() {
            let status = tokio::process::Command::new("git")
                .arg("clone")
                .arg(&git_info.repo)
                .arg(".")
                .current_dir(path)
                .output()
                .await
                .map_err(|e| SessionCreateError::Git(format!("Clone failed: {}", e)))?;

            if !status.status.success() {
                let error = String::from_utf8_lossy(&status.stderr);
                return Err(SessionCreateError::Git(format!("Clone failed: {}", error)));
            }

            // Checkout the specified ref if provided
            if let Some(r#ref) = &git_info.r#ref {
                let status = tokio::process::Command::new("git")
                    .arg("checkout")
                    .arg(r#ref)
                    .current_dir(path)
                    .output()
                    .await
                    .map_err(|e| SessionCreateError::Git(format!("Checkout failed: {}", e)))?;

                if !status.status.success() {
                    let error = String::from_utf8_lossy(&status.stderr);
                    return Err(SessionCreateError::Git(format!("Checkout failed: {}", error)));
                }
            }
        } else {
            // Initialize a new repo
            let status = tokio::process::Command::new("git")
                .arg("init")
                .current_dir(path)
                .output()
                .await
                .map_err(|e| SessionCreateError::Git(format!("Init failed: {}", e)))?;

            if !status.status.success() {
                let error = String::from_utf8_lossy(&status.stderr);
                return Err(SessionCreateError::Git(format!("Init failed: {}", error)));
            }
        }

        Ok(())
    }

    /// Setup git config
    async fn setup_git_config(
        path: &Path,
        work_secret: &WorkSecret,
    ) -> Result<(), SessionCreateError> {
        // Find git info with token
        let git_token = work_secret.sources.iter()
            .find_map(|s| s.git_info.as_ref().and_then(|g| g.token.as_ref()));

        if let Some(token) = git_token {
            // Configure credential helper
            let credential_helper = format!(
                "!f() {{ echo 'username=token'; echo 'password={}'; }}; f",
                token
            );

            let status = tokio::process::Command::new("git")
                .arg("config")
                .arg("credential.helper")
                .arg(&credential_helper)
                .current_dir(path)
                .output()
                .await;

            // Ignore errors for git config - it's optional
            let _ = status;
        }

        Ok(())
    }

    /// Setup environment variables from work secret
    fn setup_environment(work_secret: &WorkSecret) -> Result<(), SessionCreateError> {
        if let Some(env_vars) = &work_secret.environment_variables {
            for (key, value) in env_vars {
                std::env::set_var(key, value);
            }
        }

        // Setup auth tokens
        for auth in &work_secret.auth {
            match auth.r#type.as_str() {
                "anthropic" => {
                    std::env::set_var("ANTHROPIC_API_KEY", &auth.token);
                }
                "openai" => {
                    std::env::set_var("OPENAI_API_KEY", &auth.token);
                }
                _ => {}
            }
        }

        Ok(())
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
    env_vars: HashMap<String, String>,
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

    /// Get the environment variables
    pub fn env_vars(&self) -> &HashMap<String, String> {
        &self.env_vars
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
        // Try to remove the git worktree if it's a git repo
        let git_dir = path.join(".git");
        if git_dir.exists() {
            // Try git worktree remove first
            let _ = tokio::process::Command::new("git")
                .arg("worktree")
                .arg("remove")
                .arg("--force")
                .arg(path)
                .output()
                .await;
        }

        // Always try to remove the directory
        tokio::fs::remove_dir_all(path).await?;

        Ok(())
    }
}

/// Generate a UUID v4
fn generate_uuid() -> String {
    use rand::Rng;
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
        use std::fmt::Write;
        write!(&mut uuid, "{:02x}", b).unwrap();
    }
    uuid
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_spawner_new() {
        let temp_dir = std::env::temp_dir();
        let spawner = SessionSpawner::new(temp_dir, SpawnMode::SameDir);
        assert_eq!(spawner.spawn_mode, SpawnMode::SameDir);
    }

    #[test]
    fn test_generate_uuid() {
        let uuid1 = generate_uuid();
        let uuid2 = generate_uuid();
        assert_ne!(uuid1, uuid2);
        assert_eq!(uuid1.len(), 36);
    }
}
