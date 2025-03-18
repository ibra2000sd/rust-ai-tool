use crate::{Result, RustAiToolError};
use std::path::{Path, PathBuf};
use log::{debug, info, warn};
use tokio::fs;

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
                    github_clone(args[1], args[2], args.get(3).copied().unwrap_or(".")).await
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

async fn analyze_project(project_path: &str, output_format: &str) -> Result<String> {
    info!("Analyzing project at {} with output format {}", project_path, output_format);
    
    let config = load_config_for_path(project_path).await?;
    
    let results = crate::analysis::analyze_project(Path::new(project_path), &config.analysis_options)?;
    
    let output = match output_format {
        "json" => serde_json::to_string_pretty(&results)
            .map_err(|e| RustAiToolError::Other(format!("Failed to serialize results: {}", e)))?,
        "markdown" => format_analysis_results_markdown(&results),
        "console" => format_analysis_results_console(&results),
        _ => return Err(RustAiToolError::Other(format!("Unsupported output format: {}", output_format))),
    };
    
    Ok(output)
}

fn format_analysis_results_markdown(results: &[crate::analysis::AnalysisResult]) -> String {
    let mut markdown = String::new();
    
    markdown.push_str("# Rust AI Tool Analysis Results\n\n");
    
    let total_issues: usize = results.iter().map(|r| r.issues.len()).sum();
    markdown.push_str(&format!("**Total Issues Found**: {}\n\n", total_issues));
    
    for result in results {
        if result.issues.is_empty() {
            continue;
        }
        
        markdown.push_str(&format!("## {}\n\n", result.file_path.display()));
        
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

fn format_analysis_results_console(results: &[crate::analysis::AnalysisResult]) -> String {
    let mut output = String::new();
    
    let total_issues: usize = results.iter().map(|r| r.issues.len()).sum();
    output.push_str(&format!("Found {} issues in {} files\n\n", 
        total_issues,
        results.iter().filter(|r| !r.issues.is_empty()).count()
    ));
    
    for result in results {
        if result.issues.is_empty() {
            continue;
        }
        
        output.push_str(&format!("File: {}\n", result.file_path.display()));
        
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

async fn validate_fixes(project_path: &str, fixes_path: &str) -> Result<String> {
    info!("Validating fixes for project at {} using {}", project_path, fixes_path);
    
    let config = load_config_for_path(project_path).await?;
    
    let fixes_content = fs::read_to_string(fixes_path)
        .await
        .map_err(|e| RustAiToolError::Io(e))?;
    
    let fixes: Vec<crate::validation::FixToValidate> = serde_json::from_str(&fixes_content)
        .map_err(|e| RustAiToolError::Json(e))?;
    
    let validation_results = crate::validation::validate_fixes(&fixes, &config.validation_options)?;
    
    let output = format_validation_results(&validation_results);
    
    Ok(output)
}

fn format_validation_results(results: &[crate::validation::ValidationResult]) -> String {
    let mut output = String::new();
    
    let valid_count = results.iter().filter(|r| r.is_valid).count();
    let total_count = results.len();
    
    output.push_str(&format!("Validation Results: {}/{} fixes are valid\n\n", valid_count, total_count));
    
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

async fn apply_fixes(project_path: &str, fixes_path: &str, create_backup: bool) -> Result<String> {
    info!("Applying fixes to project at {} using {} (backup={})", 
          project_path, fixes_path, create_backup);
    
    let fixes_content = fs::read_to_string(fixes_path)
        .await
        .map_err(|e| RustAiToolError::Io(e))?;
    
    let modifications: Vec<crate::modification::CodeModification> = serde_json::from_str(&fixes_content)
        .map_err(|e| RustAiToolError::Json(e))?;
    
    let changes = crate::modification::apply_modifications(&modifications, create_backup)?;
    
    let report = crate::modification::create_change_report(&changes);
    
    Ok(report)
}

async fn generate_project(description: &str, output_dir: &str, name: &str) -> Result<String> {
    info!("Generating project '{}' at {} from description", name, output_dir);
    
    let config = load_default_config().await?;
    
    let project_path = crate::project_generator::generate_project_from_description(
        description,
        Path::new(output_dir),
        name,
        &config.ai_model,
    ).await?;
    
    Ok(format!("Project generated successfully at {}", project_path.display()))
}

async fn github_clone(owner: &str, repo: &str, target_dir: &str) -> Result<String> {
    info!("Cloning GitHub repository {}/{}", owner, repo);
    
    let config = load_default_config().await?;
    
    let github_config = config.github_repo.ok_or_else(|| {
        RustAiToolError::GitHub("GitHub configuration not found in config file".to_string())
    })?;
    
    let client = crate::github::GithubClient::new(
        &github_config.access_token,
        owner,
        repo,
    )?;
    
    let repo_path = client.clone_repo(None, Path::new(target_dir)).await?;
    
    Ok(format!("Repository cloned to {}", repo_path.display()))
}

async fn github_create_pr(
    owner: &str,
    repo: &str,
    branch: &str,
    title: &str,
    fixes_path: Option<&str>,
) -> Result<String> {
    info!("Creating GitHub PR for {}/{} on branch {} with title: {}", 
          owner, repo, branch, title);
    
    let config = load_default_config().await?;
    
    let github_config = config.github_repo.ok_or_else(|| {
        RustAiToolError::GitHub("GitHub configuration not found in config file".to_string())
    })?;
    
    let client = crate::github::GithubClient::new(
        &github_config.access_token,
        owner,
        repo,
    )?;
    
    let repo_info = client.get_repo_info().await?;
    
    let default_branch = &repo_info.default_branch;
    info!("Creating branch {} from {}", branch, default_branch);
    
    match client.create_branch(default_branch, branch).await {
        Ok(_) => info!("Branch created successfully"),
        Err(e) => warn!("Failed to create branch (it may already exist): {}", e),
    }
    
    let temp_dir = match std::env::temp_dir().to_str() {
        Some(dir) => dir.to_string(),
        None => return Err(RustAiToolError::Other("Failed to get temporary directory".to_string())),
    };
    
    let repo_path = client.clone_repo(Some(branch), Path::new(&temp_dir)).await?;
    
    if let Some(fixes_path_str) = fixes_path {
        let fixes_content = fs::read_to_string(fixes_path_str)
            .await
            .map_err(|e| RustAiToolError::Io(e))?;
        
        let modifications: Vec<crate::modification::CodeModification> = serde_json::from_str(&fixes_content)
            .map_err(|e| RustAiToolError::Json(e))?;
        
        info!("Modifying files:");
        for modification in &modifications {
            info!("  {}", modification.file_path.display());
        }
        
        let mut files_to_commit = Vec::new();
        for modification in &modifications {
            let file_path = &modification.file_path;
            let target_path = if file_path.is_absolute() {
                let file_name = file_path.file_name().ok_or_else(|| {
                    RustAiToolError::Modification(format!("Invalid file path: {}", file_path.display()))
                })?;
                
                let target = repo_path.join(file_name);
                
                fs::write(&target, &modification.modified_content)
                    .await
                    .map_err(|e| RustAiToolError::Io(e))?;
                
                target
            } else {
                let target = repo_path.join(file_path);
                
                if let Some(parent) = target.parent() {
                    fs::create_dir_all(parent)
                        .await
                        .map_err(|e| RustAiToolError::Io(e))?;
                }
                
                fs::write(&target, &modification.modified_content)
                    .await
                    .map_err(|e| RustAiToolError::Io(e))?;
                
                target
            };
            
            files_to_commit.push(target_path);
        }
        
        client.commit_changes(
            &repo_path,
            &files_to_commit,
            &format!("Applied fixes: {}", title),
            branch,
        ).await?;
    }
    
    let pr = client.create_pull_request(
        title,
        &format!("Automated fixes by Rust AI Tool\n\nApplied fixes"),
        branch,
        &repo_info.default_branch,
    ).await?;
    
    Ok(format!("Pull request created: {}", pr.url))
}

async fn init_config(project_path: &str) -> Result<String> {
    info!("Initializing configuration for project at {}", project_path);
    
    let config_path = Path::new(project_path).join(".rust-ai-tool.toml");
    
    if config_path.exists() {
        warn!("Configuration file already exists at {}", config_path.display());
        return Ok(format!("Configuration file already exists at {}", config_path.display()));
    }
    
    let config = create_default_config();
    
    let config_content = toml::to_string_pretty(&config)
        .map_err(|e| RustAiToolError::Other(format!("Failed to serialize configuration: {}", e)))?;
    
    fs::write(&config_path, config_content)
        .await
        .map_err(|e| RustAiToolError::Io(e))?;
    
    Ok(format!("Configuration file created at {}", config_path.display()))
}

async fn load_config_for_path(project_path: &str) -> Result<crate::Config> {
    let config_path = Path::new(project_path).join(".rust-ai-tool.toml");
    
    if config_path.exists() {
        let config_content = fs::read_to_string(&config_path)
            .await
            .map_err(|e| RustAiToolError::Io(e))?;
        
        let mut config: crate::Config = toml::from_str(&config_content)
            .map_err(|e| RustAiToolError::Other(format!("Failed to parse configuration: {}", e)))?;
        
        config.project_path = Path::new(project_path).to_path_buf();
        
        Ok(config)
    } else {
        let mut config = create_default_config();
        config.project_path = Path::new(project_path).to_path_buf();
        
        Ok(config)
    }
}

async fn load_default_config() -> Result<crate::Config> {
    let current_dir = std::env::current_dir()
        .map_err(|e| RustAiToolError::Io(e))?;
    
    load_config_for_path(current_dir.to_str().unwrap_or(".")).await
}

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

pub fn create_terminal_ui() -> Result<()> {
    Ok(())
}

pub fn create_progress_display(operation: &str, total: u64) -> Result<ProgressHandler> {
    println!("Starting {}...", operation);
    
    Ok(ProgressHandler {
        operation: operation.to_string(),
        total,
        current: 0,
    })
}

pub struct ProgressHandler {
    operation: String,
    total: u64,
    current: u64,
}

impl ProgressHandler {
    pub fn update(&mut self, current: u64) {
        self.current = current;
        
        let percentage = if self.total > 0 {
            (self.current as f64 / self.total as f64 * 100.0) as u64
        } else {
            0
        };
        
        println!("{}: {}% ({}/{})", self.operation, percentage, self.current, self.total);
    }
    
    pub fn increment(&mut self) {
        self.update(self.current + 1);
    }
    
    pub fn complete(&mut self) {
        self.update(self.total);
        println!("{} completed.", self.operation);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_init_config() {
        let dir = std::env::temp_dir();
        let path = dir.to_str().unwrap();
        
        let result = init_config(path).await.unwrap();
        assert!(result.contains("Configuration file created"));
        
        let config_path = Path::new(path).join(".rust-ai-tool.toml");
        assert!(config_path.exists());
        
        // Clean up
        fs::remove_file(config_path).await.ok();
    }
}