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
use log::{debug, info, warn, error};
use serde::{Serialize, Deserialize};

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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
        info!("Cloning repository {}/{} to {}", 
              self.owner, self.repo, target_dir.display());
              
        let repo_url = format!("https://github.com/{}/{}.git", self.owner, self.repo);
        let output_dir = target_dir.join(&self.repo);
        
        let mut cmd = Command::new("git");
        cmd.arg("clone");
        
        // If a specific branch is requested
        if let Some(branch_name) = branch {
            debug!("Cloning branch: {}", branch_name);
            cmd.arg("--branch").arg(branch_name);
        }
        
        cmd.arg("--single-branch")
           .arg(&repo_url)
           .arg(&output_dir);
        
        debug!("Running git command: {:?}", cmd);
        
        let status = cmd.status().await.map_err(|e| RustAiToolError::Io(e))?;
        
        if !status.success() {
            return Err(RustAiToolError::GitHub(format!(
                "Failed to clone repository: {} (exit code: {:?})",
                repo_url,
                status.code()
            )));
        }
        
        info!("Successfully cloned repository to {}", output_dir.display());
        Ok(output_dir)
    }
    
    /// Get repository information
    ///
    /// # Returns
    ///
    /// Information about the repository
    pub async fn get_repo_info(&self) -> Result<RepoInfo> {
        info!("Getting information for repository {}/{}", self.owner, self.repo);
        
        let repo = self.client
            .repos(&self.owner, &self.repo)
            .get()
            .await
            .map_err(|e| RustAiToolError::GitHub(e.to_string()))?;
        
        let info = RepoInfo {
            owner: self.owner.clone(),
            repo: self.repo.clone(),
            default_branch: repo.default_branch.unwrap_or_else(|| "main".to_string()),
            is_fork: repo.fork.unwrap_or(false),
            description: repo.description,
        };
        
        debug!("Repository info: {:?}", info);
        Ok(info)
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
        info!("Creating branch {} from {}", new_branch, base_branch);
        
        // Get the SHA of the latest commit on the base branch
        let reference = self.client
            .repos(&self.owner, &self.repo)
            .get_ref(&format!("heads/{}", base_branch))
            .await
            .map_err(|e| RustAiToolError::GitHub(e.to_string()))?;
        
        let sha = reference.object.sha;
        debug!("Base branch SHA: {}", sha);
        
        // Create a new reference (branch) using that SHA
        self.client
            .repos(&self.owner, &self.repo)
            .create_ref(&format!("refs/heads/{}", new_branch), &sha)
            .await
            .map_err(|e| RustAiToolError::GitHub(e.to_string()))?;
        
        info!("Successfully created branch {}", new_branch);
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
        info!("Creating pull request: {} ({} -> {})", title, head, base);
        
        let pull_request = self.client
            .pulls(&self.owner, &self.repo)
            .create(title, head, base)
            .body(body)
            .send()
            .await
            .map_err(|e| RustAiToolError::GitHub(e.to_string()))?;
        
        let pr_info = PullRequestInfo {
            number: pull_request.number,
            title: pull_request.title.unwrap_or_else(|| title.to_string()),
            url: pull_request.html_url.map_or_else(
                || format!("https://github.com/{}/{}/pull/{}", self.owner, self.repo, pull_request.number),
                |url| url.to_string(),
            ),
            is_merged: false,
            state: pull_request.state.unwrap_or_else(|| "open".to_string()),
        };
        
        info!("Successfully created pull request #{}: {}", pr_info.number, pr_info.url);
        Ok(pr_info)
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
        info!("Committing {} files to branch {}", files.len(), branch);
        
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
            debug!("Staging file: {}", file.display());
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
        
        info!("Successfully committed and pushed changes");
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
        info!("Adding comment to PR #{}", pr_number);
        
        self.client
            .issues(&self.owner, &self.repo)
            .create_comment(pr_number, comment)
            .await
            .map_err(|e| RustAiToolError::GitHub(e.to_string()))?;
        
        info!("Successfully added comment to PR #{}", pr_number);
        Ok(())
    }
    
    /// Get pull request information
    ///
    /// # Arguments
    ///
    /// * `pr_number` - Pull request number
    ///
    /// # Returns
    ///
    /// Information about the pull request
    pub async fn get_pull_request(&self, pr_number: u64) -> Result<PullRequestInfo> {
        info!("Getting information for PR #{}", pr_number);
        
        let pull_request = self.client
            .pulls(&self.owner, &self.repo)
            .get(pr_number)
            .await
            .map_err(|e| RustAiToolError::GitHub(e.to_string()))?;
        
        let pr_info = PullRequestInfo {
            number: pull_request.number,
            title: pull_request.title.unwrap_or_else(|| "No title".to_string()),
            url: pull_request.html_url.map_or_else(
                || format!("https://github.com/{}/{}/pull/{}", self.owner, self.repo, pull_request.number),
                |url| url.to_string(),
            ),
            is_merged: pull_request.merged.unwrap_or(false),
            state: pull_request.state.unwrap_or_else(|| "unknown".to_string()),
        };
        
        debug!("PR info: {:?}", pr_info);
        Ok(pr_info)
    }
    
    /// List pull requests
    ///
    /// # Arguments
    ///
    /// * `state` - Pull request state (open, closed, all)
    ///
    /// # Returns
    ///
    /// List of pull requests
    pub async fn list_pull_requests(&self, state: &str) -> Result<Vec<PullRequestInfo>> {
        info!("Listing {} pull requests", state);
        
        let pull_requests = self.client
            .pulls(&self.owner, &self.repo)
            .list()
            .state(state)
            .send()
            .await
            .map_err(|e| RustAiToolError::GitHub(e.to_string()))?;
        
        let mut prs = Vec::new();
        for pr in pull_requests.items {
            prs.push(PullRequestInfo {
                number: pr.number,
                title: pr.title.unwrap_or_else(|| "No title".to_string()),
                url: pr.html_url.map_or_else(
                    || format!("https://github.com/{}/{}/pull/{}", self.owner, self.repo, pr.number),
                    |url| url.to_string(),
                ),
                is_merged: pr.merged.unwrap_or(false),
                state: pr.state.unwrap_or_else(|| "unknown".to_string()),
            });
        }
        
        info!("Found {} {} pull requests", prs.len(), state);
        Ok(prs)
    }
    
    /// Merge a pull request
    ///
    /// # Arguments
    ///
    /// * `pr_number` - Pull request number
    /// * `commit_message` - Commit message
    ///
    /// # Returns
    ///
    /// Success status
    pub async fn merge_pull_request(&self, pr_number: u64, commit_message: &str) -> Result<()> {
        info!("Merging PR #{}", pr_number);
        
        self.client
            .pulls(&self.owner, &self.repo)
            .merge(pr_number)
            .commit_message(commit_message)
            .send()
            .await
            .map_err(|e| RustAiToolError::GitHub(e.to_string()))?;
        
        info!("Successfully merged PR #{}", pr_number);
        Ok(())
    }
    
    /// Get file content from a repository
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the file
    /// * `branch` - Branch to get the file from (or None for default)
    ///
    /// # Returns
    ///
    /// File content
    pub async fn get_file_content(&self, path: &str, branch: Option<&str>) -> Result<String> {
        info!("Getting content of file: {}", path);
        
        let content = self.client
            .repos(&self.owner, &self.repo)
            .get_content()
            .path(path)
            .r#ref(branch.unwrap_or(""))
            .send()
            .await
            .map_err(|e| RustAiToolError::GitHub(e.to_string()))?;
        
        match content {
            models::repos::ContentItems::File(file) => {
                if let Some(content) = file.content {
                    // Decode base64 content
                    let decoded = base64::decode(&content.replace("\n", ""))
                        .map_err(|e| RustAiToolError::GitHub(format!("Failed to decode base64: {}", e)))?;
                    
                    let content = String::from_utf8(decoded)
                        .map_err(|e| RustAiToolError::GitHub(format!("Failed to decode UTF-8: {}", e)))?;
                    
                    Ok(content)
                } else {
                    Err(RustAiToolError::GitHub("File content is empty".to_string()))
                }
            },
            _ => Err(RustAiToolError::GitHub(format!("Path is not a file: {}", path))),
        }
    }
    
    /// Create or update a file in a repository
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the file
    /// * `content` - File content
    /// * `commit_message` - Commit message
    /// * `branch` - Branch to update (or None for default)
    ///
    /// # Returns
    ///
    /// Success status
    pub async fn create_or_update_file(&self, path: &str, content: &str, commit_message: &str, branch: Option<&str>) -> Result<()> {
        info!("Creating or updating file: {}", path);
        
        // Get the current file to get its SHA (if it exists)
        let sha = match self.client
            .repos(&self.owner, &self.repo)
            .get_content()
            .path(path)
            .r#ref(branch.unwrap_or(""))
            .send()
            .await
        {
            Ok(models::repos::ContentItems::File(file)) => file.sha,
            _ => None, // File doesn't exist yet
        };
        
        // Encode content as base64
        let encoded = base64::encode(content);
        
        // Create or update the file
        self.client
            .repos(&self.owner, &self.repo)
            .create_or_update_file(path, commit_message, &encoded)
            .branch(branch.unwrap_or(""))
            .sha(sha.as_deref().unwrap_or(""))
            .send()
            .await
            .map_err(|e| RustAiToolError::GitHub(e.to_string()))?;
        
        info!("Successfully created or updated file: {}", path);
        Ok(())
    }
    
    /// Create an issue
    ///
    /// # Arguments
    ///
    /// * `title` - Issue title
    /// * `body` - Issue description
    /// * `labels` - Issue labels
    ///
    /// # Returns
    ///
    /// Issue number
    pub async fn create_issue(&self, title: &str, body: &str, labels: &[String]) -> Result<u64> {
        info!("Creating issue: {}", title);
        
        let issue = self.client
            .issues(&self.owner, &self.repo)
            .create(title)
            .body(body)
            .labels(labels.to_vec())
            .send()
            .await
            .map_err(|e| RustAiToolError::GitHub(e.to_string()))?;
        
        info!("Successfully created issue #{}", issue.number);
        Ok(issue.number)
    }
    
    /// List repository branches
    ///
    /// # Returns
    ///
    /// List of branch names
    pub async fn list_branches(&self) -> Result<Vec<String>> {
        info!("Listing branches for {}/{}", self.owner, self.repo);
        
        let branches = self.client
            .repos(&self.owner, &self.repo)
            .list_branches()
            .send()
            .await
            .map_err(|e| RustAiToolError::GitHub(e.to_string()))?;
        
        let branch_names = branches.items
            .into_iter()
            .map(|branch| branch.name)
            .collect();
        
        Ok(branch_names)
    }
    
    /// Compare two branches or commits
    ///
    /// # Arguments
    ///
    /// * `base` - Base branch or commit
    /// * `head` - Head branch or commit
    ///
    /// # Returns
    ///
    /// List of files changed
    pub async fn compare_branches(&self, base: &str, head: &str) -> Result<Vec<String>> {
        info!("Comparing {} with {}", base, head);
        
        let comparison = self.client
            .repos(&self.owner, &self.repo)
            .compare(base, head)
            .await
            .map_err(|e| RustAiToolError::GitHub(e.to_string()))?;
        
        let files = comparison.files
            .into_iter()
            .map(|file| file.filename)
            .collect();
        
        Ok(files)
    }
}