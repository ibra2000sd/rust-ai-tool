//! GitHub integration module
//!
//! This module provides functionality for interacting with GitHub:
//! - Cloning repositories
//! - Creating pull requests with suggested fixes
//! - Managing issues and comments
//! - Repository analysis

use crate::{GitHubRepo, Result, RustAiToolError};
use octocrab::{models, Octocrab};
use std::path::{Path, PathBuf};
use tokio::process::Command;

/// GitHub client for interacting with the GitHub API
pub struct GithubClient {
    /// Octocrab client for GitHub API
    client: Octocrab,
    
    /// Repository owner
    owner: String,
    
    /// Repository name
    repo: String,
}

/// Information about a GitHub repository
#[derive(Debug, Clone)]
pub struct RepoInfo {
    /// Repository owner
    pub owner: String,
    
    /// Repository name
    pub repo: String,
    
    /// Default branch
    pub default_branch: String,
    
    /// Whether the repository is a fork
    pub is_fork: bool,
    
    /// Repository description
    pub description: Option<String>,
}

/// Information about a GitHub pull request
#[derive(Debug, Clone)]
pub struct PullRequestInfo {
    /// Pull request number
    pub number: u64,
    
    /// Pull request title
    pub title: String,
    
    /// Pull request URL
    pub url: String,
    
    /// Whether the pull request is merged
    pub is_merged: bool,
    
    /// Pull request state
    pub state: String,
}

impl GithubClient {
    /// Create a new GitHub client
    ///
    /// # Arguments
    ///
    /// * `token` - GitHub access token
    /// * `owner` - Repository owner
    /// * `repo` - Repository name
    ///
    /// # Returns
    ///
    /// A new GitHub client
    pub fn new(token: &str, owner: &str, repo: &str) -> Result<Self> {
        let client = Octocrab::builder()
            .personal_token(token.to_string())
            .build()
            .map_err(|e| RustAiToolError::GitHub(e.to_string()))?;
        
        Ok(Self {
            client,
            owner: owner.to_string(),
            repo: repo.to_string(),
        })
    }
    
    /// Create a new GitHub client from a GitHubRepo
    ///
    /// # Arguments
    ///
    /// * `repo` - GitHub repository information
    ///
    /// # Returns
    ///
    /// A new GitHub client
    pub fn from_repo(repo: &GitHubRepo) -> Result<Self> {
        Self::new(&repo.access_token, &repo.owner, &repo.name)
    }
    
    /// Clone a repository to a local directory
    ///
    /// # Arguments
    ///
    /// * `branch` - Branch to clone (or None for default)
    /// * `target_dir` - Directory to clone to
    ///
    /// # Returns
    ///
    /// Path to the cloned repository
    pub async fn clone_repo(&self, branch: Option<&str>, target_dir: &Path) -> Result<PathBuf> {
        let repo_url = format!("https://github.com/{}/{}.git", self.owner, self.repo);
        let output_dir = target_dir.join(&self.repo);
        
        let mut cmd = Command::new("git");
        cmd.arg("clone");
        
        // If a specific branch is requested
        if let Some(branch_name) = branch {
            cmd.arg("--branch").arg(branch_name);
        }
        
        cmd.arg("--single-branch")
           .arg(&repo_url)
           .arg(&output_dir);
        
        let status = cmd.status().await.map_err(|e| RustAiToolError::Io(e))?;
        
        if !status.success() {
            return Err(RustAiToolError::GitHub(format!(
                "Failed to clone repository: {} (exit code: {:?})",
                repo_url,
                status.code()
            )));
        }
        
        Ok(output_dir)
    }
    
    /// Get repository information
    ///
    /// # Returns
    ///
    /// Information about the repository
    pub async fn get_repo_info(&self) -> Result<RepoInfo> {
        let repo = self.client
            .repos(&self.owner, &self.repo)
            .get()
            .await
            .map_err(|e| RustAiToolError::GitHub(e.to_string()))?;
        
        Ok(RepoInfo {
            owner: self.owner.clone(),
            repo: self.repo.clone(),
            default_branch: repo.default_branch.unwrap_or_else(|| "main".to_string()),
            is_fork: repo.fork.unwrap_or(false),
            description: repo.description,
        })
    }
    
    /// Create a new branch
    ///
    /// # Arguments
    ///
    /// * `base_branch` - Branch to create from
    /// * `new_branch` - Name of the new branch
    ///
    /// # Returns
    ///
    /// Success status
    pub async fn create_branch(&self, base_branch: &str, new_branch: &str) -> Result<()> {
        // Get the SHA of the latest commit on the base branch
        let reference = self.client
            .repos(&self.owner, &self.repo)
            .get_ref(&format!("heads/{}", base_branch))
            .await
            .map_err(|e| RustAiToolError::GitHub(e.to_string()))?;
        
        let sha = reference.object.sha;
        
        // Create a new reference (branch) using that SHA
        self.client
            .repos(&self.owner, &self.repo)
            .create_ref(&format!("refs/heads/{}", new_branch), &sha)
            .await
            .map_err(|e| RustAiToolError::GitHub(e.to_string()))?;
        
        Ok(())
    }
    
    /// Create a pull request
    ///
    /// # Arguments
    ///
    /// * `title` - Pull request title
    /// * `body` - Pull request description
    /// * `head` - Head branch
    /// * `base` - Base branch
    ///
    /// # Returns
    ///
    /// Information about the created pull request
    pub async fn create_pull_request(
        &self,
        title: &str,
        body: &str,
        head: &str,
        base: &str,
    ) -> Result<PullRequestInfo> {
        let pull_request = self.client
            .pulls(&self.owner, &self.repo)
            .create(title, head, base)
            .body(body)
            .send()
            .await
            .map_err(|e| RustAiToolError::GitHub(e.to_string()))?;
        
        Ok(PullRequestInfo {
            number: pull_request.number,
            title: pull_request.title.unwrap_or_else(|| title.to_string()),
            url: pull_request.html_url.map_or_else(
                || format!("https://github.com/{}/{}/pull/{}", self.owner, self.repo, pull_request.number),
                |url| url.to_string(),
            ),
            is_merged: false,
            state: pull_request.state.unwrap_or_else(|| "open".to_string()),
        })
    }
    
    /// Commit changes to a repository
    ///
    /// # Arguments
    ///
    /// * `repo_path` - Path to the local repository
    /// * `files` - List of files to commit
    /// * `message` - Commit message
    /// * `branch` - Branch to commit to
    ///
    /// # Returns
    ///
    /// Success status
    pub async fn commit_changes(
        &self,
        repo_path: &Path,
        files: &[PathBuf],
        message: &str,
        branch: &str,
    ) -> Result<()> {
        // Change to the repository directory
        let current_dir = std::env::current_dir().map_err(|e| RustAiToolError::Io(e))?;
        std::env::set_current_dir(repo_path).map_err(|e| RustAiToolError::Io(e))?;
        
        // Make sure we're on the right branch
        let switch_result = Command::new("git")
            .args(&["checkout", branch])
            .status()
            .await
            .map_err(|e| RustAiToolError::Io(e))?;
        
        if !switch_result.success() {
            std::env::set_current_dir(current_dir).ok();
            return Err(RustAiToolError::GitHub(format!(
                "Failed to switch to branch: {} (exit code: {:?})",
                branch,
                switch_result.code()
            )));
        }
        
        // Stage the files
        for file in files {
            let add_result = Command::new("git")
                .args(&["add", &file.to_string_lossy()])
                .status()
                .await
                .map_err(|e| RustAiToolError::Io(e))?;
            
            if !add_result.success() {
                std::env::set_current_dir(current_dir).ok();
                return Err(RustAiToolError::GitHub(format!(
                    "Failed to stage file: {} (exit code: {:?})",
                    file.display(),
                    add_result.code()
                )));
            }
        }
        
        // Commit the changes
        let commit_result = Command::new("git")
            .args(&["commit", "-m", message])
            .status()
            .await
            .map_err(|e| RustAiToolError::Io(e))?;
        
        if !commit_result.success() {
            std::env::set_current_dir(current_dir).ok();
            return Err(RustAiToolError::GitHub(format!(
                "Failed to commit changes (exit code: {:?})",
                commit_result.code()
            )));
        }
        
        // Push the changes
        let push_result = Command::new("git")
            .args(&["push", "origin", branch])
            .status()
            .await
            .map_err(|e| RustAiToolError::Io(e))?;
        
        if !push_result.success() {
            std::env::set_current_dir(current_dir).ok();
            return Err(RustAiToolError::GitHub(format!(
                "Failed to push changes (exit code: {:?})",
                push_result.code()
            )));
        }
        
        // Return to the original directory
        std::env::set_current_dir(current_dir).map_err(|e| RustAiToolError::Io(e))?;
        
        Ok(())
    }
    
    /// Add a comment to a pull request
    ///
    /// # Arguments
    ///
    /// * `pr_number` - Pull request number
    /// * `comment` - Comment text
    ///
    /// # Returns
    ///
    /// Success status
    pub async fn add_pr_comment(&self, pr_number: u64, comment: &str) -> Result<()> {
        self.client
            .issues(&self.owner, &self.repo)
            .create_comment(pr_number, comment)
            .await
            .map_err(|e| RustAiToolError::GitHub(e.to_string()))?;
        
        Ok(())
    }
}