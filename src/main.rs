use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use log::{debug, error, info, warn};
use rust_ai_tool::{
    analysis::{self, analyze_project, AnalysisResult},
    cli,
    github::GithubClient,
    modification::{apply_modifications, CodeModification, create_change_report},
    project_generator::{generate_project_from_description, ProjectConfig, ProjectTemplate},
    validation::{self, validate_fixes, FixToValidate, ValidationResult},
    AiModelConfig, AiModelType, AnalysisOptions, Config, GitHubRepo, ValidationOptions,
};
use std::fs;
use std::path::{Path, PathBuf};
use tokio::runtime::Runtime;

/// Rust AI-Powered Project Analyzer & Code Refactoring Tool
#[derive(Parser, Debug)]
#[clap(author, version, about)]
struct Cli {
    /// Subcommand to execute
    #[clap(subcommand)]
    command: Commands,

    /// Enable verbose logging
    #[clap(short, long)]
    verbose: bool,

    /// Configuration file path
    #[clap(short, long, default_value = ".rust-ai-tool.toml")]
    config: PathBuf,
}

/// Supported commands
#[derive(Subcommand, Debug)]
enum Commands {
    /// Analyze a Rust project and suggest improvements
    Analyze {
        /// Path to Rust project
        #[clap(default_value = ".")]
        project_path: PathBuf,

        /// Output format (json, markdown, console)
        #[clap(short, long, default_value = "console")]
        output: String,

        /// Output file path (if not specified, output to stdout)
        #[clap(short, long)]
        file: Option<PathBuf>,
    },

    /// Validate suggested fixes for a Rust project
    Validate {
        /// Path to Rust project
        #[clap(default_value = ".")]
        project_path: PathBuf,

        /// Path to JSON file containing suggested fixes
        #[clap(short, long)]
        fixes: PathBuf,
    },

    /// Apply suggested fixes to a Rust project
    Apply {
        /// Path to Rust project
        #[clap(default_value = ".")]
        project_path: PathBuf,

        /// Path to JSON file containing suggested fixes
        #[clap(short, long)]
        fixes: PathBuf,

        /// Create a backup before applying fixes
        #[clap(short, long)]
        backup: bool,
    },

    /// Generate a new Rust project from description
    Generate {
        /// Project description
        #[clap(short, long)]
        description: String,

        /// Output directory
        #[clap(short, long)]
        output: PathBuf,

        /// Project name
        #[clap(short, long)]
        name: String,
    },

    /// GitHub integration commands
    GitHub {
        /// GitHub subcommand
        #[clap(subcommand)]
        command: GitHubCommands,
    },

    /// Initialize a new Rust AI Tool configuration
    Init {
        /// Path to Rust project
        #[clap(default_value = ".")]
        project_path: PathBuf,
    },
}

/// GitHub-specific commands
#[derive(Subcommand, Debug)]
enum GitHubCommands {
    /// Create a pull request with suggested fixes
    CreatePr {
        /// Repository owner
        #[clap(short, long)]
        owner: String,

        /// Repository name
        #[clap(short, long)]
        repo: String,

        /// Branch name
        #[clap(short, long)]
        branch: String,

        /// Pull request title
        #[clap(short, long)]
        title: String,

        /// Path to fixes JSON file
        #[clap(short, long)]
        fixes: PathBuf,
    },

    /// Clone and analyze a GitHub repository
    Analyze {
        /// Repository owner
        #[clap(short, long)]
        owner: String,

        /// Repository name
        #[clap(short, long)]
        repo: String,

        /// Branch name
        #[clap(short, long, default_value = "main")]
        branch: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logger
    let log_level = if cli.verbose {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Info
    };
    
    env_logger::Builder::new()
        .filter_level(log_level)
        .format_timestamp(None)
        .init();

    debug!("Parsed CLI arguments: {:#?}", cli);

    // Load configuration or create default
    let config = match load_config(&cli.config) {
        Ok(config) => {
            debug!("Loaded configuration from {}", cli.config.display());
            config
        }
        Err(e) => {
            warn!("Failed to load configuration: {}", e);
            warn!("Using default configuration");
            create_default_config()
        }
    };

    debug!("Using configuration: {:#?}", config);

    // Execute command
    match &cli.command {
        Commands::Analyze {
            project_path,
            output,
            file,
        } => {
            info!("Analyzing project at {}", project_path.display());
            
            let results = analyze_project(project_path, &config.analysis_options)
                .context("Failed to analyze project")?;
            
            let output_content = format_analysis_results(&results, output)?;
            
            if let Some(output_file) = file {
                fs::write(output_file, &output_content)
                    .context(format!("Failed to write output to {}", output_file.display()))?;
                
                info!("Analysis results written to {}", output_file.display());
            } else {
                println!("{}", output_content);
            }
            
            info!("Analysis complete");
        }
        Commands::Validate { project_path, fixes } => {
            info!(
                "Validating fixes for project at {} using {}",
                project_path.display(),
                fixes.display()
            );
            
            let fixes_content = fs::read_to_string(fixes)
                .context(format!("Failed to read fixes file: {}", fixes.display()))?;
            
            let fixes_to_validate: Vec<FixToValidate> = serde_json::from_str(&fixes_content)
                .context("Failed to parse fixes JSON")?;
            
            let validation_results = validate_fixes(&fixes_to_validate, &config.validation_options)
                .context("Failed to validate fixes")?;
            
            let valid_count = validation_results.iter().filter(|r| r.is_valid).count();
            let total_count = validation_results.len();
            
            println!("Validation complete: {}/{} fixes are valid", valid_count, total_count);
            
            for (i, result) in validation_results.iter().enumerate() {
                if !result.is_valid {
                    println!("Fix #{} for {} is invalid:", i + 1, result.file_path.display());
                    for msg in &result.messages {
                        println!("  - {}: {}", msg.message_type, msg.text);
                    }
                }
            }
        }
        Commands::Apply {
            project_path,
            fixes,
            backup,
        } => {
            info!(
                "Applying fixes to project at {} using {}",
                project_path.display(),
                fixes.display()
            );
            
            if *backup {
                info!("Creating backup before applying fixes");
            }
            
            let fixes_content = fs::read_to_string(fixes)
                .context(format!("Failed to read fixes file: {}", fixes.display()))?;
            
            let modifications: Vec<CodeModification> = serde_json::from_str(&fixes_content)
                .context("Failed to parse fixes JSON")?;
            
            let changes = apply_modifications(&modifications, *backup)
                .context("Failed to apply modifications")?;
            
            let report = create_change_report(&changes);
            println!("{}", report);
            
            info!("Successfully applied {} changes", changes.len());
        }
        Commands::Generate {
            description,
            output,
            name,
        } => {
            info!(
                "Generating project '{}' at {} from description",
                name,
                output.display()
            );
            
            if !output.exists() {
                fs::create_dir_all(output)
                    .context(format!("Failed to create output directory: {}", output.display()))?;
            }
            
            let project_path = generate_project_from_description(
                description,
                output,
                name,
                &config.ai_model
            ).await.context("Failed to generate project")?;
            
            info!("Project generated at {}", project_path.display());
        }
        Commands::GitHub { command } => match command {
            GitHubCommands::CreatePr {
                owner,
                repo,
                branch,
                title,
                fixes,
            } => {
                info!(
                    "Creating PR for {}/{} on branch {} with title: {}",
                    owner, repo, branch, title
                );
                
                let github_config = config.github_repo.as_ref()
                    .context("GitHub configuration not found in config file")?;
                
                let github = GithubClient::new(&github_config.access_token, owner, repo)
                    .context("Failed to create GitHub client")?;
                
                // Get repo info
                let repo_info = github.get_repo_info().await
                    .context("Failed to get repository information")?;
                
                // Create a new branch if it doesn't exist
                info!("Creating branch {} from {}", branch, repo_info.default_branch);
                let _ = github.create_branch(&repo_info.default_branch, branch).await;
                
                // Clone the repository to a temporary directory
                let temp_dir = tempfile::tempdir().context("Failed to create temporary directory")?;
                let repo_path = github.clone_repo(Some(branch), temp_dir.path()).await
                    .context("Failed to clone repository")?;
                
                // Read fixes
                let fixes_content = fs::read_to_string(fixes)
                    .context(format!("Failed to read fixes file: {}", fixes.display()))?;
                
                let modifications: Vec<CodeModification> = serde_json::from_str(&fixes_content)
                    .context("Failed to parse fixes JSON")?;
                
                // Apply modifications
                let changed_files: Vec<PathBuf> = modifications.iter()
                    .map(|m| {
                        let rel_path = m.file_path.strip_prefix(project_path).unwrap_or(&m.file_path);
                        repo_path.join(rel_path)
                    })
                    .collect();
                
                // Commit and push changes
                github.commit_changes(
                    &repo_path,
                    &changed_files,
                    &format!("Applied fixes: {}", title),
                    branch,
                ).await.context("Failed to commit changes")?;
                
                // Create pull request
                let pr = github.create_pull_request(
                    title,
                    &format!("Automatically generated fixes by Rust AI Tool"),
                    branch,
                    &repo_info.default_branch,
                ).await.context("Failed to create pull request")?;
                
                println!("Pull request created successfully: {}", pr.url);
            }
            GitHubCommands::Analyze {
                owner,
                repo,
                branch,
            } => {
                info!("Analyzing GitHub repository {}/{} on branch {}", owner, repo, branch);
                
                let github_config = config.github_repo.as_ref()
                    .context("GitHub configuration not found in config file")?;
                
                let github = GithubClient::new(&github_config.access_token, owner, repo)
                    .context("Failed to create GitHub client")?;
                
                // Clone the repository to a temporary directory
                let temp_dir = tempfile::tempdir().context("Failed to create temporary directory")?;
                let repo_path = github.clone_repo(Some(branch), temp_dir.path()).await
                    .context("Failed to clone repository")?;
                
                // Run analysis
                let results = analyze_project(&repo_path, &config.analysis_options)
                    .context("Failed to analyze project")?;
                
                // Output results
                let output_content = format_analysis_results(&results, "markdown")?;
                println!("{}", output_content);
                
                info!("GitHub repository analysis complete");
            }
        },
        Commands::Init { project_path } => {
            info!("Initializing configuration for project at {}", project_path.display());
            
            let config_path = project_path.join(".rust-ai-tool.toml");
            
            if config_path.exists() {
                warn!("Configuration file already exists at {}", config_path.display());
                warn!("Use --force to overwrite existing configuration");
                return Ok(());
            }
            
            let config = create_default_config();
            let config_content = toml::to_string_pretty(&config)
                .context("Failed to serialize configuration")?;
            
            fs::write(&config_path, config_content)
                .context(format!("Failed to write configuration to {}", config_path.display()))?;
            
            info!("Configuration initialized at {}", config_path.display());
        }
    }

    Ok(())
}

/// Load the configuration from a file
fn load_config(config_path: &PathBuf) -> Result<Config> {
    // Check if the file exists
    if !config_path.exists() {
        return Err(anyhow::anyhow!("Configuration file not found: {}", config_path.display()));
    }

    // Read and parse the configuration file
    let config_content = fs::read_to_string(config_path)
        .context(format!("Failed to read configuration file: {}", config_path.display()))?;
    
    let mut config: Config = toml::from_str(&config_content)
        .context("Failed to parse configuration file")?;
    
    // Set project path to the parent directory of the config file
    if let Some(parent) = config_path.parent() {
        config.project_path = parent.to_path_buf();
    } else {
        config.project_path = std::env::current_dir()
            .context("Failed to get current directory")?;
    }
    
    Ok(config)
}

/// Create a default configuration
fn create_default_config() -> Config {
    Config {
        project_path: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        github_repo: None,
        ai_model: AiModelConfig {
            model_type: AiModelType::Claude,
            api_key: String::new(),
            api_base_url: None,
        },
        analysis_options: AnalysisOptions {
            run_clippy: true,
            use_rust_analyzer: true,
            custom_rules: Vec::new(),
        },
        validation_options: ValidationOptions {
            syntax_only: false,
            tauri_compatibility: true,
            security_validation: true,
        },
    }
}

/// Format analysis results as the specified output format
fn format_analysis_results(results: &[AnalysisResult], format: &str) -> Result<String> {
    match format.to_lowercase().as_str() {
        "json" => {
            let json = serde_json::to_string_pretty(results)
                .context("Failed to serialize analysis results to JSON")?;
            Ok(json)
        }
        "markdown" => {
            let mut markdown = String::new();
            markdown.push_str("# Rust AI Tool Analysis Results\n\n");
            
            let issue_count: usize = results.iter()
                .map(|r| r.issues.len())
                .sum();
            
            markdown.push_str(&format!("**Total Issues Found**: {}\n\n", issue_count));
            
            for result in results {
                if result.issues.is_empty() {
                    continue;
                }
                
                markdown.push_str(&format!("## {}\n\n", result.file_path.display()));
                
                for issue in &result.issues {
                    markdown.push_str(&format!("### {}:{}-{}\n\n", 
                        issue.file_path.display(),
                        issue.line_start,
                        issue.line_end
                    ));
                    
                    markdown.push_str(&format!("**Category**: {}\n\n", format!("{:?}", issue.category)));
                    markdown.push_str(&format!("**Severity**: {}\n\n", format!("{:?}", issue.severity)));
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
            
            Ok(markdown)
        }
        "console" => {
            let mut output = String::new();
            let issue_count: usize = results.iter()
                .map(|r| r.issues.len())
                .sum();
            
            output.push_str(&format!("Total Issues Found: {}\n\n", issue_count));
            
            for result in results {
                if result.issues.is_empty() {
                    continue;
                }
                
                output.push_str(&format!("File: {}\n", result.file_path.display()));
                
                for (i, issue) in result.issues.iter().enumerate() {
                    output.push_str(&format!("Issue #{}: {}:{}-{} ({:?}, {:?})\n", 
                        i + 1,
                        issue.file_path.display(),
                        issue.line_start,
                        issue.line_end,
                        issue.category,
                        issue.severity
                    ));
                    
                    output.push_str(&format!("  {}\n", issue.message));
                    
                    if let Some(fix) = &issue.suggested_fix {
                        output.push_str("  Suggested Fix:\n");
                        for line in fix.replacement_code.lines() {
                            output.push_str(&format!("    {}\n", line));
                        }
                        output.push_str(&format!("  Confidence: {}%\n", fix.confidence));
                    }
                    
                    output.push_str("\n");
                }
                
                output.push_str("---\n\n");
            }
            
            Ok(output)
        }
        _ => Err(anyhow::anyhow!("Unsupported output format: {}", format))
    }
}