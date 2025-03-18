//! Command-line interface module
//!
//! This module provides functionality for the CLI interface:
//! - Command execution
//! - Terminal UI
//! - Progress reporting
//! - User interaction

use crate::{Result, RustAiToolError};
use std::path::Path;
use log::{debug, info, warn, error};
use serde::{Serialize, Deserialize};
use tokio::fs;

/// Execute a CLI command
///
/// # Arguments
///
/// * `command` - Command to execute
/// * `args` - Command arguments
///
/// # Returns
///
/// Success status
pub async fn execute_command(command: &str, args: &[&str]) -> Result<String> {
    debug!("Executing command: {} with args: {:?}", command, args);
    
    match command {
        "analyze" => {
            let project_path = if args.is_empty() { "." } else { args[0] };
            let output_format = if args.len() > 1 { args[1] } else { "console" };
            
            analyze_project(project_path, output_format).await
        }
        "validate" => {
            if args.len() < 2 {
                return Err(RustAiToolError::Validation(
                    "validate command requires project path and fixes file".to_string(),
                ));
            }
            validate_fixes(args[0], args[1]).await
        }
        "apply" => {
            if args.len() < 2 {
                return Err(RustAiToolError::Modification(
                    "apply command requires project path and fixes file".to_string(),
                ));
            }
            let create_backup = args.len() > 2 && args[2] == "--backup";
            apply_fixes(args[0], args[1], create_backup).await
        }
        "generate" => {
            if args.len() < 3 {
                return Err(RustAiToolError::ProjectGeneration(
                    "generate command requires description, output directory, and name".to_string(),
                ));
            }
            generate_project(args[0], args[1], args[2]).await
        }
        "github" => {
            if args.is_empty() {
                return Err(RustAiToolError::GitHub(
                    "github command requires a subcommand".to_string(),
                ));
            }
            
            match args[0] {
                "clone" => {
                    if args.len() < 3 {
                        return Err(RustAiToolError::GitHub(
                            "github clone command requires owner, repo, and target directory".to_string(),
                        ));
                    }
                    github_clone(args[1], args[2], args.get(3).copied()).await
                }
                "create-pr" => {
                    if args.len() < 5 {
                        return Err(RustAiToolError::GitHub(
                            "github create-pr command requires owner, repo, branch, title, and fixes file".to_string(),
                        ));
                    }
                    github_create_pr(args[1], args[2], args[3], args[4], args.get(5).copied()).await
                }
                _ => Err(RustAiToolError::Other(format!("Unknown github subcommand: {}", args[0]))),
            }
        }
        "init" => {
            let project_path = if args.is_empty() { "." } else { args[0] };
            init_config(project_path).await
        }
        _ => Err(RustAiToolError::Other(format!("Unknown command: {}", command))),
    }
}

/// Analyze a Rust project
///
/// # Arguments
///
/// * `project_path` - Path to the project
/// * `output_format` - Output format (json, markdown, console)
///
/// # Returns
///
/// Analysis results
async fn analyze_project(project_path: &str, output_format: &str) -> Result<String> {
    info!("Analyzing project at {} with output format {}", project_path, output_format);
    
    // Load the configuration
    let config = load_config_for_path(project_path).await?;
    
    // Run the analysis
    let results = crate::analysis::analyze_project(Path::new(project_path), &config.analysis_options)?;
    
    // Format the results
    let output = match output_format {
        "json" => serde_json::to_string_pretty(&results)
            .map_err(|e| RustAiToolError::Other(format!("Failed to serialize results: {}", e)))?,
        "markdown" => format_analysis_results_markdown(&results),
        "console" => format_analysis_results_console(&results),
        _ => return Err(RustAiToolError::Other(format!("Unsupported output format: {}", output_format))),
    };
    
    Ok(output)
}

/// Format analysis results as markdown
///
/// # Arguments
///
/// * `results` - Analysis results
///
/// # Returns
///
/// Markdown-formatted results
fn format_analysis_results_markdown(results: &[crate::analysis::AnalysisResult]) -> String {
    let mut markdown = String::new();
    
    markdown.push_str("# Rust AI Tool Analysis Results\n\n");
    
    // Count total issues
    let total_issues: usize = results.iter().map(|r| r.issues.len()).sum();
    markdown.push_str(&format!("**Total Issues Found**: {}\n\n", total_issues));
    
    // Process each file
    for result in results {
        if result.issues.is_empty() {
            continue;
        }
        
        markdown.push_str(&format!("## {}\n\n", result.file_path.display()));
        
        // Process each issue
        for issue in &result.issues {
            markdown.push_str(&format!("### Issue at {}:{}-{}\n\n", 
                issue.file_path.display(), 
                issue.line_start, 
                issue.line_end
            ));
            
            markdown.push_str(&format!("**Category**: {:?}\n\n", issue.category));
            markdown.push_str(&format!("**Severity**: {:?}\n\n", issue.severity));
            markdown.push_str(&format!("**Message**: {}\n\n", issue.message));
            
            if let Some(fix) = &issue.suggested_fix {
                markdown.push_str("**Suggested Fix**:\n\n");
                markdown.push_str("```rust\n");
                markdown.push_str(&fix.replacement_code);
                markdown.push_str("\n```\n\n");
                markdown.push_str(&format!("Confidence: {}%\n\n", fix.confidence));
            }
            
            markdown.push_str("---\n\n");
        }
    }
    
    markdown
}

/// Format analysis results for console output
///
/// # Arguments
///
/// * `results` - Analysis results
///
/// # Returns
///
/// Console-formatted results
fn format_analysis_results_console(results: &[crate::analysis::AnalysisResult]) -> String {
    let mut output = String::new();
    
    // Count total issues
    let total_issues: usize = results.iter().map(|r| r.issues.len()).sum();
    output.push_str(&format!("Found {} issues in {} files\n\n", 
        total_issues,
        results.iter().filter(|r| !r.issues.is_empty()).count()
    ));
    
    // Process each file
    for result in results {
        if result.issues.is_empty() {
            continue;
        }
        
        output.push_str(&format!("File: {}\n", result.file_path.display()));
        
        // Process each issue
        for (i, issue) in result.issues.iter().enumerate() {
            output.push_str(&format!("  Issue #{}: {}:{}-{} ({:?}, {:?})\n", 
                i + 1,
                issue.file_path.display(),
                issue.line_start,
                issue.line_end,
                issue.category,
                issue.severity
            ));
            
            output.push_str(&format!("    Message: {}\n", issue.message));
            
            if let Some(fix) = &issue.suggested_fix {
                output.push_str("    Suggested Fix:\n");
                
                // Format the replacement code with indentation
                for line in fix.replacement_code.lines() {
                    output.push_str(&format!("      {}\n", line));
                }
                
                output.push_str(&format!("    Confidence: {}%\n", fix.confidence));
            }
            
            output.push_str("\n");
        }
        
        output.push_str("---\n\n");
    }
    
    output
}

/// Validate suggested fixes
///
/// # Arguments
///
/// * `project_path` - Path to the project
/// * `fixes_path` - Path to the fixes file
///
/// # Returns
///
/// Validation results
async fn validate_fixes(project_path: &str, fixes_path: &str) -> Result<String> {
    info!("Validating fixes for project at {} using {}", project_path, fixes_path);
    
    // Load the configuration
    let config = load_config_for_path(project_path).await?;
    
    // Load the fixes
    let fixes_content = fs::read_to_string(fixes_path)
        .await
        .map_err(|e| RustAiToolError::Io(e))?;
    
    let fixes: Vec<crate::validation::FixToValidate> = serde_json::from_str(&fixes_content)
        .map_err(|e| RustAiToolError::Json(e))?;
    
    // Validate the fixes
    let validation_results = crate::validation::validate_fixes(&fixes, &config.validation_options)?;
    
    // Format the results
    let output = format_validation_results(&validation_results);
    
    Ok(output)
}

/// Format validation results
///
/// # Arguments
///
/// * `results` - Validation results
///
/// # Returns
///
/// Formatted results
fn format_validation_results(results: &[crate::validation::ValidationResult]) -> String {
    let mut output = String::new();
    
    // Count valid and invalid fixes
    let valid_count = results.iter().filter(|r| r.is_valid).count();
    let total_count = results.len();
    
    output.push_str(&format!("Validation Results: {}/{} fixes are valid\n\n", valid_count, total_count));
    
    // Process each result
    for (i, result) in results.iter().enumerate() {
        output.push_str(&format!("Fix #{} for {}: {}\n", 
            i + 1,
            result.file_path.display(),
            if result.is_valid { "VALID" } else { "INVALID" }
        ));
        
        if !result.is_valid {
            output.push_str(&format!("  Severity: {:?}\n", result.severity));
            
            for msg in &result.messages {
                if msg.message_type == crate::validation::ValidationMessageType::Error {
                    output.push_str(&format!("  - ERROR: {}\n", msg.text));
                }
            }
        }
        
        output.push_str("\n");
    }
    
    output
}

/// Apply suggested fixes
///
/// # Arguments
///
/// * `project_path` - Path to the project
/// * `fixes_path` - Path to the fixes file
/// * `create_backup` - Whether to create backups
///
/// # Returns
///
/// Application results
async fn apply_fixes(project_path: &str, fixes_path: &str, create_backup: bool) -> Result<String> {
    info!("Applying fixes to project at {} using {} (backup={})", 
          project_path, fixes_path, create_backup);
    
    // Load the fixes
    let fixes_content = fs::read_to_string(fixes_path)
        .await
        .map_err(|e| RustAiToolError::Io(e))?;
    
    let modifications: Vec<crate::modification::CodeModification> = serde_json::from_str(&fixes_content)
        .map_err(|e| RustAiToolError::Json(e))?;
    
    // Apply the modifications
    let changes = crate::modification::apply_modifications(&modifications, create_backup)?;
    
    // Generate a report
    let report = crate::modification::create_change_report(&changes);
    
    Ok(report)
}

/// Generate a Rust project
///
/// # Arguments
///
/// * `description` - Project description
/// * `output_dir` - Output directory
/// * `name` - Project name
///
/// # Returns
///
/// Generation results
async fn generate_project(description: &str, output_dir: &str, name: &str) -> Result<String> {
    info!("Generating project '{}' at {} from description", name, output_dir);
    
    // Load default config for AI model
    let config = load_default_config().await?;
    
    // Generate the project
    let project_path = crate::project_generator::generate_project_from_description(
        description,
        Path::new(output_dir),
        name,
        &config.ai_model,
    ).await?;
    
    Ok(format!("Project generated successfully at {}", project_path.display()))
}

/// Clone a GitHub repository
///
/// # Arguments
///
/// * `owner` - Repository owner
/// * `repo` - Repository name
/// * `target_dir` - Target directory
///
/// # Returns
///
/// Clone results
async fn github_clone(owner: &str, repo: &str, target_dir: Option<&str>) -> Result<String> {
    info!("Cloning GitHub repository {}/{}", owner, repo);
    
    // Load config to get GitHub token
    let config = load_default_config().await?;
    
    let github_config = config.github_repo.ok_or_else(|| {
        RustAiToolError::GitHub("GitHub configuration not found in config file".to_string())
    })?;
    
    // Create GitHub client
    let client = crate::github::GithubClient::new(
        &github_config.access_token,
        owner,
        repo,
    )?;
    
    // Clone the repository
    let target = target_dir.unwrap_or(".");
    let repo_path = client.clone_repo(None, Path::new(target)).await?;
    
    Ok(format!("Repository cloned to {}", repo_path.display()))
}

/// Create a GitHub pull request
///
/// # Arguments
///
/// * `owner` - Repository owner
/// * `repo` - Repository name
/// * `branch` - Branch name
/// * `title` - Pull request title
/// * `fixes_path` - Path to fixes file
///
/// # Returns
///
/// Pull request results
async fn github_create_pr(
    owner: &str,
    repo: &str,
    branch: &str,
    title: &str,
    fixes_path: &str,
) -> Result<String> {
    info!("Creating GitHub PR for {}/{} on branch {} with title: {}", 
          owner, repo, branch, title);
    
    // Load config to get GitHub token
    let config = load_default_config().await?;
    
    let github_config = config.github_repo.ok_or_else(|| {
        RustAiToolError::GitHub("GitHub configuration not found in config file".to_string())
    })?;
    
    // Create GitHub client
    let client = crate::github::GithubClient::new(
        &github_config.access_token,
        owner,
        repo,
    )?;
    
    // Get repository info to determine default branch
    let repo_info = client.get_repo_info().await?;
    
    // Create a new branch if it doesn't exist
    let default_branch = &repo_info.default_branch;
    info!("Creating branch {} from {}", branch, default_branch);
    
    match client.create_branch(default_branch, branch).await {
        Ok(_) => info!("Branch created successfully"),
        Err(e) => warn!("Failed to create branch (it may already exist): {}", e),
    }
    
    // Clone the repository to a temporary directory
    let temp_dir = tempfile::tempdir()
        .map_err(|e| RustAiToolError::Other(format!("Failed to create temporary directory: {}", e)))?;
    
    let repo_path = client.clone_repo(Some(branch), temp_dir.path()).await?;
    
    // Load the fixes
    let fixes_content = fs::read_to_string(fixes_path)
        .await
        .map_err(|e| RustAiToolError::Io(e))?;
    
    let modifications: Vec<crate::modification::CodeModification> = serde_json::from_str(&fixes_content)
        .map_err(|e| RustAiToolError::Json(e))?;
    
    // Apply modifications to the local repository
    let paths: Vec<_> = modifications.iter()
        .map(|m| &m.file_path)
        .collect();
    
    // Log the files that will be modified
    info!("Modifying files:");
    for path in &paths {
        info!("  {}", path.display());
    }
    
    // Make path adjustments - if file_path is absolute, make it relative to the repo
    let mut files_to_commit = Vec::new();
    for modification in &modifications {
        let file_path = &modification.file_path;
        let target_path = if file_path.is_absolute() {
            // Try to make it relative to the repo
            let file_name = file_path.file_name().ok_or_else(|| {
                RustAiToolError::Modification(format!("Invalid file path: {}", file_path.display()))
            })?;
            
            let target = repo_path.join(file_name);
            
            // Write the modified content
            fs::write(&target, &modification.modified_content)
                .await
                .map_err(|e| RustAiToolError::Io(e))?;
            
            target
        } else {
            // The path is already relative, so just join it with the repo path
            let target = repo_path.join(file_path);
            
            // Create parent directories if they don't exist
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)
                    .await
                    .map_err(|e| RustAiToolError::Io(e))?;
            }
            
            // Write the modified content
            fs::write(&target, &modification.modified_content)
                .await
                .map_err(|e| RustAiToolError::Io(e))?;
            
            target
        };
        
        files_to_commit.push(target_path);
    }
    
    // Commit and push changes
    client.commit_changes(
        &repo_path,
        &files_to_commit,
        &format!("Applied fixes: {}", title),
        branch,
    ).await?;
    
    // Create pull request
    let pr = client.create_pull_request(
        title,
        &format!("Automated fixes by Rust AI Tool\n\nApplied {} fixes", modifications.len()),
        branch,
        &repo_info.default_branch,
    ).await?;
    
    Ok(format!("Pull request created: {}", pr.url))
}

/// Initialize a configuration file
///
/// # Arguments
///
/// * `project_path` - Path to the project
///
/// # Returns
///
/// Initialization results
async fn init_config(project_path: &str) -> Result<String> {
    info!("Initializing configuration for project at {}", project_path);
    
    let config_path = Path::new(project_path).join(".rust-ai-tool.toml");
    
    // Check if the file already exists
    if config_path.exists() {
        warn!("Configuration file already exists at {}", config_path.display());
        return Ok(format!("Configuration file already exists at {}", config_path.display()));
    }
    
    // Create a default configuration
    let config = create_default_config();
    
    // Serialize the configuration
    let config_content = toml::to_string_pretty(&config)
        .map_err(|e| RustAiToolError::Other(format!("Failed to serialize configuration: {}", e)))?;
    
    // Write the configuration file
    fs::write(&config_path, config_content)
        .await
        .map_err(|e| RustAiToolError::Io(e))?;
    
    Ok(format!("Configuration file created at {}", config_path.display()))
}

/// Load configuration for a project path
///
/// # Arguments
///
/// * `project_path` - Path to the project
///
/// # Returns
///
/// Project configuration
async fn load_config_for_path(project_path: &str) -> Result<crate::Config> {
    let config_path = Path::new(project_path).join(".rust-ai-tool.toml");
    
    if config_path.exists() {
        let config_content = fs::read_to_string(&config_path)
            .await
            .map_err(|e| RustAiToolError::Io(e))?;
        
        let mut config: crate::Config = toml::from_str(&config_content)
            .map_err(|e| RustAiToolError::Other(format!("Failed to parse configuration: {}", e)))?;
        
        // Set the project path
        config.project_path = Path::new(project_path).to_path_buf();
        
        Ok(config)
    } else {
        // If no config file exists, create a default one
        let mut config = create_default_config();
        config.project_path = Path::new(project_path).to_path_buf();
        
        Ok(config)
    }
}

/// Load default configuration
///
/// # Returns
///
/// Default configuration
async fn load_default_config() -> Result<crate::Config> {
    // Check if config exists in current directory
    let current_dir = std::env::current_dir()
        .map_err(|e| RustAiToolError::Io(e))?;
    
    load_config_for_path(current_dir.to_str().unwrap_or(".")).await
}

/// Create a default configuration
///
/// # Returns
///
/// Default configuration
fn create_default_config() -> crate::Config {
    crate::Config {
        project_path: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        github_repo: None,
        ai_model: crate::AiModelConfig {
            model_type: crate::AiModelType::Claude,
            api_key: String::new(),
            api_base_url: None,
        },
        analysis_options: crate::AnalysisOptions {
            run_clippy: true,
            use_rust_analyzer: true,
            custom_rules: Vec::new(),
        },
        validation_options: crate::ValidationOptions {
            syntax_only: false,
            tauri_compatibility: true,
            security_validation: true,
        },
    }
}

/// Create interactive terminal UI for the application
///
/// # Returns
///
/// Success status
pub fn create_terminal_ui() -> Result<()> {
    // This is a placeholder for a terminal UI
    // In a real implementation, this would create a more sophisticated UI
    // using a library like tui-rs
    
    Ok(())
}

/// Display progress for long-running operations
///
/// # Arguments
///
/// * `operation` - Operation description
/// * `total` - Total number of steps
///
/// # Returns
///
/// Progress handler
pub fn create_progress_display(operation: &str, total: u64) -> Result<ProgressHandler> {
    // This is a placeholder for a progress display
    // In a real implementation, this would create a more sophisticated progress bar
    // using a library like indicatif
    
    println!("Starting {}...", operation);
    
    Ok(ProgressHandler {
        operation: operation.to_string(),
        total,
        current: 0,
    })
}

/// Progress handler for long-running operations
pub struct ProgressHandler {
    /// Operation description
    operation: String,
    
    /// Total number of steps
    total: u64,
    
    /// Current step
    current: u64,
}

impl ProgressHandler {
    /// Update progress
    ///
    /// # Arguments
    ///
    /// * `current` - Current step
    pub fn update(&mut self, current: u64) {
        self.current = current;
        
        let percentage = if self.total > 0 {
            (self.current as f64 / self.total as f64 * 100.0) as u64
        } else {
            0
        };
        
        println!("{}: {}% ({}/{})", self.operation, percentage, self.current, self.total);
    }
    
    /// Increment progress
    pub fn increment(&mut self) {
        self.update(self.current + 1);
    }
    
    /// Complete progress
    pub fn complete(&mut self) {
        self.update(self.total);
        println!("{} completed.", self.operation);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[tokio::test]
    async fn test_init_config() {
        let dir = tempdir().unwrap();
        let path = dir.path().to_str().unwrap();
        
        let result = init_config(path).await.unwrap();
        assert!(result.contains("Configuration file created"));
        
        let config_path = Path::new(path).join(".rust-ai-tool.toml");
        assert!(config_path.exists());
    }
}