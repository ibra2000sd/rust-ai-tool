use crate::{GitHubRepo, Result, RustAiToolError};
use octocrab::{models, Octocrab, params};
use std::path::{Path, PathBuf};
use tokio::process::Command;
use log::{debug, info};
use serde::{Serialize, Deserialize};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

pub struct GithubClient {
    client: Octocrab,
    owner: String,
    repo: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoInfo {
    pub owner: String,
    pub repo: String,
    pub default_branch: String,
    pub is_fork: bool,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequestInfo {
    pub number: u64,
    pub title: String,
    pub url: String,
    pub is_merged: bool,
    pub state: String,
}

impl GithubClient {
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
    
    pub fn from_repo(repo: &GitHubRepo) -> Result<Self> {
        Self::new(&repo.access_token, &repo.owner, &repo.name)
    }
    
    pub async fn clone_repo(&self, branch: Option<&str>, target_dir: &Path) -> Result<PathBuf> {
        info!("Cloning repository {}/{} to {}", 
              self.owner, self.repo, target_dir.display());
              
        let repo_url = format!("https://github.com/{}/{}.git", self.owner, self.repo);
        let output_dir = target_dir.join(&self.repo);
        
        let mut cmd = Command::new("git");
        cmd.arg("clone");
        
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
    
    pub async fn create_branch(&self, base_branch: &str, new_branch: &str) -> Result<()> {
        info!("Creating branch {} from {}", new_branch, base_branch);
        
        // Get the SHA of the latest commit on the base branch
        let reference = self.client
            .repos(&self.owner, &self.repo)
            .get_ref(&format!("heads/{}", base_branch))
            .await
            .map_err(|e| RustAiToolError::GitHub(e.to_string()))?;
        
        debug!("Base branch ref: {:?}", reference);
        
        // Use the SHA directly from the reference object
        let sha = reference.object.sha;
        debug!("Base branch SHA: {}", sha);
        
        // Create a new reference (branch) using that SHA
        self.client
            .repos(&self.owner, &self.repo)
            .create_ref(&format!("refs/heads/{}", new_branch), sha)
            .await
            .map_err(|e| RustAiToolError::GitHub(e.to_string()))?;
        
        info!("Successfully created branch {}", new_branch);
        Ok(())
    }
    
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
            is_merged: false, // Default to false since we just created it
            state: pull_request.state.map_or_else(
                || "open".to_string(),
                |s| format!("{:?}", s).to_lowercase() // Use debug formatting and convert to lowercase
            ),
        };
        
        info!("Successfully created pull request #{}: {}", pr_info.number, pr_info.url);
        Ok(pr_info)
    }
    
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
    
    pub async fn get_pull_request(&self, pr_number: u64) -> Result<PullRequestInfo> {
        info!("Getting information for PR #{}", pr_number);
        
        let pull_request = self.client
            .pulls(&self.owner, &self.repo)
            .get(pr_number)
            .await
            .map_err(|e| RustAiToolError::GitHub(e.to_string()))?;
        
        // Check if the PR is merged with a separate API call
        let is_merged = self.client
            .pulls(&self.owner, &self.repo)
            .is_merged(pr_number)
            .await
            .unwrap_or(false);
        
        let pr_info = PullRequestInfo {
            number: pull_request.number,
            title: pull_request.title.unwrap_or_else(|| "No title".to_string()),
            url: pull_request.html_url.map_or_else(
                || format!("https://github.com/{}/{}/pull/{}", self.owner, self.repo, pull_request.number),
                |url| url.to_string(),
            ),
            is_merged,
            state: pull_request.state.map_or_else(
                || "unknown".to_string(),
                |s| format!("{:?}", s).to_lowercase()
            ),
        };
        
        debug!("PR info: {:?}", pr_info);
        Ok(pr_info)
    }
    
    pub async fn list_pull_requests(&self, state: &str) -> Result<Vec<PullRequestInfo>> {
        info!("Listing {} pull requests", state);
        
        // Convert string state to the enum that octocrab expects
        let state_param = match state {
            "open" => params::State::Open,
            "closed" => params::State::Closed,
            "all" => params::State::All,
            _ => params::State::Open, // Default to open
        };
        
        let pull_requests = self.client
            .pulls(&self.owner, &self.repo)
            .list()
            .state(state_param)
            .send()
            .await
            .map_err(|e| RustAiToolError::GitHub(e.to_string()))?;
        
        let mut prs = Vec::new();
        for pr in pull_requests.items {
            // Check if the PR is merged with a separate API call
            let is_merged = self.client
                .pulls(&self.owner, &self.repo)
                .is_merged(pr.number)
                .await
                .unwrap_or(false);
            
            prs.push(PullRequestInfo {
                number: pr.number,
                title: pr.title.unwrap_or_else(|| "No title".to_string()),
                url: pr.html_url.map_or_else(
                    || format!("https://github.com/{}/{}/pull/{}", self.owner, self.repo, pr.number),
                    |url| url.to_string(),
                ),
                is_merged,
                state: pr.state.map_or_else(
                    || "unknown".to_string(),
                    |s| format!("{:?}", s).to_lowercase()
                ),
            });
        }
        
        info!("Found {} {} pull requests", prs.len(), state);
        Ok(prs)
    }
    
    pub async fn merge_pull_request(&self, pr_number: u64, commit_message: &str) -> Result<()> {
        info!("Merging PR #{}", pr_number);
        
        self.client
            .pulls(&self.owner, &self.repo)
            .merge(pr_number)
            .message(commit_message)
            .send()
            .await
            .map_err(|e| RustAiToolError::GitHub(e.to_string()))?;
        
        info!("Successfully merged PR #{}", pr_number);
        Ok(())
    }
    
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
            // In newer octocrab versions, the ContentItems enum has different variants
            // Try different variants depending on your octocrab version
            models::repos::ContentItems::File(file) => {
                if let Some(content) = file.content {
                    // Decode base64 content
                    let content = content.replace("\n", "");
                    
                    // Use the BASE64 engine from the base64 crate
                    let decoded = BASE64.decode(content)
                        .map_err(|e| RustAiToolError::GitHub(format!("Failed to decode base64: {}", e)))?;
                    
                    let content = String::from_utf8(decoded)
                        .map_err(|e| RustAiToolError::GitHub(format!("Failed to decode UTF-8: {}", e)))?;
                    
                    Ok(content)
                } else {
                    Err(RustAiToolError::GitHub("File content is empty".to_string()))
                }
            },
            // If your octocrab version uses a different enum variant, add it here
            _ => Err(RustAiToolError::GitHub(format!("Path is not a file: {}", path))),
        }
    }
    
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
        
        // Encode content as base64 using the BASE64 engine
        let encoded = BASE64.encode(content);
        
        // Create or update the file
        if let Some(sha_str) = sha {
            // Update existing file
            self.client
                .repos(&self.owner, &self.repo)
                .update_file(path, commit_message, &encoded, &sha_str)
                .branch(branch.unwrap_or(""))
                .send()
                .await
                .map_err(|e| RustAiToolError::GitHub(e.to_string()))?;
        } else {
            // Create new file
            self.client
                .repos(&self.owner, &self.repo)
                .create_file(path, commit_message, &encoded)
                .branch(branch.unwrap_or(""))
                .send()
                .await
                .map_err(|e| RustAiToolError::GitHub(e.to_string()))?;
        }
        
        info!("Successfully created or updated file: {}", path);
        Ok(())
    }
    
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
    
    pub async fn compare_branches(&self, base: &str, head: &str) -> Result<Vec<String>> {
        info!("Comparing {} with {}", base, head);
        
        // Use the custom endpoint API from octocrab
        let endpoint = format!("repos/{}/{}/compare/{}...{}", 
            self.owner, self.repo, base, head);
        
        let response: serde_json::Value = self.client
            .get(&endpoint, None::<&()>)
            .await
            .map_err(|e| RustAiToolError::GitHub(e.to_string()))?;
        
        // Extract filenames from the response
        let files = response["files"]
            .as_array()
            .map(|array| {
                array.iter()
                    .filter_map(|file| file["filename"].as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        
        Ok(files)
    }
}