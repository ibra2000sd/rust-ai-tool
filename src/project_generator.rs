//! Project generation module
//!
//! This module provides functionality to generate Rust projects:
//! - Create new Rust projects from templates
//! - Generate code based on AI descriptions
//! - Create project scaffolding with best practices

use crate::{Result, RustAiToolError, AiModelConfig};
use std::path::{Path, PathBuf};
use tokio::process::Command;

/// Project template
#[derive(Debug, Clone, PartialEq)]
pub enum ProjectTemplate {
    /// Basic Rust binary
    Basic,
    
    /// Rust library
    Library,
    
    /// Command-line application
    Cli,
    
    /// Web service with Actix
    WebService,
    
    /// Tauri desktop application
    TauriApp,
    
    /// Custom template
    Custom(String),
}

/// Configuration for project generation
#[derive(Debug, Clone)]
pub struct ProjectConfig {
    /// Project name
    pub name: String,
    
    /// Project description
    pub description: String,
    
    /// Project template
    pub template: ProjectTemplate,
    
    /// Author name
    pub author: String,
    
    /// Crate type (bin, lib, etc.)
    pub crate_type: String,
    
    /// Output directory
    pub output_dir: PathBuf,
    
    /// Whether to initialize a Git repository
    pub init_git: bool,
    
    /// Additional dependencies to include
    pub dependencies: Vec<String>,
    
    /// AI model configuration for code generation
    pub ai_model: Option<AiModelConfig>,
}

/// Generate a new Rust project from a description
///
/// # Arguments
///
/// * `description` - Project description
/// * `output_dir` - Output directory
/// * `name` - Project name
/// * `ai_model` - AI model configuration
///
/// # Returns
///
/// Path to the generated project
pub async fn generate_project_from_description(
    description: &str,
    output_dir: &Path,
    name: &str,
    ai_model: &AiModelConfig,
) -> Result<PathBuf> {
    log::info!("Generating project from description: {}", description);
    
    // Create a project configuration based on the description
    let config = analyze_description(description, output_dir, name, ai_model).await?;
    
    // Generate the project
    generate_project(&config).await
}

/// Analyze a project description to determine configuration
///
/// # Arguments
///
/// * `description` - Project description
/// * `output_dir` - Output directory
/// * `name` - Project name
/// * `ai_model` - AI model configuration
///
/// # Returns
///
/// Project configuration
async fn analyze_description(
    description: &str,
    output_dir: &Path,
    name: &str,
    ai_model: &AiModelConfig,
) -> Result<ProjectConfig> {
    // In a real implementation, this would use AI to analyze the description
    // For now, we'll use a simple heuristic
    
    let template = if description.contains("web") || description.contains("server") || description.contains("api") {
        ProjectTemplate::WebService
    } else if description.contains("desktop") || description.contains("gui") || description.contains("tauri") {
        ProjectTemplate::TauriApp
    } else if description.contains("cli") || description.contains("command") {
        ProjectTemplate::Cli
    } else if description.contains("library") || description.contains("lib") {
        ProjectTemplate::Library
    } else {
        ProjectTemplate::Basic
    };
    
    let crate_type = match template {
        ProjectTemplate::Library => "lib".to_string(),
        _ => "bin".to_string(),
    };
    
    let dependencies = extract_dependencies(description);
    
    let author = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "Rust AI Tool User".to_string());
    
    Ok(ProjectConfig {
        name: name.to_string(),
        description: description.to_string(),
        template,
        author,
        crate_type,
        output_dir: output_dir.to_path_buf(),
        init_git: true,
        dependencies,
        ai_model: Some(ai_model.clone()),
    })
}

/// Extract dependencies from a project description
///
/// # Arguments
///
/// * `description` - Project description
///
/// # Returns
///
/// List of dependencies
fn extract_dependencies(description: &str) -> Vec<String> {
    let mut dependencies = Vec::new();
    
    // Common crates to detect
    let known_crates = [
        "serde", "tokio", "reqwest", "clap", "hyper", "actix-web",
        "rocket", "diesel", "sqlx", "rusqlite", "mongodb", "tauri",
        "egui", "wgpu", "image", "anyhow", "thiserror", "tracing",
        "log", "env_logger", "rand", "chrono", "uuid", "regex",
    ];
    
    for crate_name in &known_crates {
        if description.contains(crate_name) {
            dependencies.push(crate_name.to_string());
        }
    }
    
    dependencies
}

/// Generate a Rust project
///
/// # Arguments
///
/// * `config` - Project configuration
///
/// # Returns
///
/// Path to the generated project
pub async fn generate_project(config: &ProjectConfig) -> Result<PathBuf> {
    let project_dir = config.output_dir.join(&config.name);
    
    // Create the project directory
    std::fs::create_dir_all(&project_dir)
        .map_err(|e| RustAiToolError::Io(e))?;
    
    // Initialize Cargo project
    let cargo_init_result = Command::new("cargo")
        .arg("init")
        .arg("--name")
        .arg(&config.name)
        .arg(if config.crate_type == "lib" { "--lib" } else { "--bin" })
        .current_dir(&project_dir)
        .status()
        .await
        .map_err(|e| RustAiToolError::Io(e))?;
    
    if !cargo_init_result.success() {
        return Err(RustAiToolError::ProjectGeneration(format!(
            "Failed to initialize Cargo project (exit code: {:?})",
            cargo_init_result.code()
        )));
    }
    
    // Update Cargo.toml
    update_cargo_toml(&project_dir, config).await?;
    
    // Generate project files based on template
    generate_project_files(&project_dir, config).await?;
    
    // Initialize Git repository if requested
    if config.init_git {
        init_git_repository(&project_dir).await?;
    }
    
    Ok(project_dir)
}

/// Update Cargo.toml with project configuration
///
/// # Arguments
///
/// * `project_dir` - Project directory
/// * `config` - Project configuration
///
/// # Returns
///
/// Success status
async fn update_cargo_toml(project_dir: &Path, config: &ProjectConfig) -> Result<()> {
    let cargo_toml_path = project_dir.join("Cargo.toml");
    
    // Read the existing Cargo.toml
    let cargo_toml = std::fs::read_to_string(&cargo_toml_path)
        .map_err(|e| RustAiToolError::Io(e))?;
    
    // Parse it
    let mut cargo_doc = cargo_toml.parse::<toml::Document>()
        .map_err(|e| RustAiToolError::ProjectGeneration(format!("Failed to parse Cargo.toml: {}", e)))?;
    
    // Update package metadata
    if let Some(package) = cargo_doc.get_mut("package") {
        if let Some(table) = package.as_table_mut() {
            // Update description
            table.insert("description", toml::value::Value::String(config.description.clone()));
            
            // Update author
            table.insert("authors", toml::value::Value::Array(vec![
                toml::value::Value::String(config.author.clone())
            ]));
            
            // Add license
            table.insert("license", toml::value::Value::String("MIT".to_string()));
            
            // Add repository (default to GitHub)
            table.insert(
                "repository",
                toml::value::Value::String(format!("https://github.com/username/{}", config.name)),
            );
            
            // Add keywords
            let keywords = extract_keywords(&config.description);
            table.insert(
                "keywords",
                toml::value::Value::Array(
                    keywords
                        .iter()
                        .map(|k| toml::value::Value::String(k.clone()))
                        .collect(),
                ),
            );
        }
    }
    
    // Add dependencies
    if let Some(deps) = cargo_doc.get_mut("dependencies") {
        if let Some(table) = deps.as_table_mut() {
            for dep in &config.dependencies {
                table.insert(dep, toml::value::Value::String("*".to_string()));
            }
        }
    }
    
    // Write the updated Cargo.toml
    std::fs::write(&cargo_toml_path, cargo_doc.to_string())
        .map_err(|e| RustAiToolError::Io(e))?;
    
    Ok(())
}

/// Extract keywords from a project description
///
/// # Arguments
///
/// * `description` - Project description
///
/// # Returns
///
/// List of keywords
fn extract_keywords(description: &str) -> Vec<String> {
    let mut keywords = Vec::new();
    
    // Common keywords to extract
    let common_keywords = [
        "web", "cli", "api", "server", "client", "database", "gui", "game",
        "tool", "utility", "library", "framework", "desktop", "mobile",
    ];
    
    for keyword in &common_keywords {
        if description.to_lowercase().contains(keyword) && !keywords.contains(&keyword.to_string()) {
            keywords.push(keyword.to_string());
        }
    }
    
    // Limit to 5 keywords (crates.io limit)
    keywords.truncate(5);
    
    keywords
}

/// Generate project files based on template
///
/// # Arguments
///
/// * `project_dir` - Project directory
/// * `config` - Project configuration
///
/// # Returns
///
/// Success status
async fn generate_project_files(project_dir: &Path, config: &ProjectConfig) -> Result<()> {
    match &config.template {
        ProjectTemplate::Basic => generate_basic_project(project_dir, config).await?,
        ProjectTemplate::Library => generate_library_project(project_dir, config).await?,
        ProjectTemplate::Cli => generate_cli_project(project_dir, config).await?,
        ProjectTemplate::WebService => generate_web_service_project(project_dir, config).await?,
        ProjectTemplate::TauriApp => generate_tauri_project(project_dir, config).await?,
        ProjectTemplate::Custom(template_path) => {
            generate_custom_project(project_dir, config, template_path).await?
        },
    }
    
    Ok(())
}

/// Generate a basic Rust project
///
/// # Arguments
///
/// * `project_dir` - Project directory
/// * `config` - Project configuration
///
/// # Returns
///
/// Success status
async fn generate_basic_project(project_dir: &Path, config: &ProjectConfig) -> Result<()> {
    // The most basic project will have been initialized by cargo init
    // We can add some additional files or customizations here
    
    // Create a README.md
    let readme_path = project_dir.join("README.md");
    let readme_content = format!(
        "# {}\n\n{}\n\n## Getting Started\n\n```bash\ncargo run\n```\n",
        config.name, config.description
    );
    
    std::fs::write(&readme_path, readme_content)
        .map_err(|e| RustAiToolError::Io(e))?;
    
    // Create a .gitignore
    let gitignore_path = project_dir.join(".gitignore");
    let gitignore_content = r#"/target
**/*.rs.bk
Cargo.lock
"#;
    
    std::fs::write(&gitignore_path, gitignore_content)
        .map_err(|e| RustAiToolError::Io(e))?;
    
    // If we have an AI model, we can also generate some initial code
    if let Some(ai_model) = &config.ai_model {
        // Generate main.rs content with AI
        generate_main_rs_with_ai(project_dir, config, ai_model).await?;
    }
    
    Ok(())
}

/// Generate a library Rust project
///
/// # Arguments
///
/// * `project_dir` - Project directory
/// * `config` - Project configuration
///
/// # Returns
///
/// Success status
async fn generate_library_project(project_dir: &Path, config: &ProjectConfig) -> Result<()> {
    // Create a basic project first
    generate_basic_project(project_dir, config).await?;
    
    // Create src/lib.rs with better documentation
    let lib_rs_path = project_dir.join("src").join("lib.rs");
    let lib_rs_content = format!(
        r#"//! # {}
//!
//! {}
//!
//! ## Examples
//!
//! ```
//! // Example code will go here
//! ```

/// Example function
///
/// # Examples
///
/// ```
/// // Example usage
/// ```
pub fn example_function() -> bool {{
    true
}}

#[cfg(test)]
mod tests {{
    use super::*;

    #[test]
    fn it_works() {{
        assert_eq!(example_function(), true);
    }}
}}
"#,
        config.name, config.description
    );
    
    std::fs::write(&lib_rs_path, lib_rs_content)
        .map_err(|e| RustAiToolError::Io(e))?;
    
    // Create examples directory with a simple example
    let examples_dir = project_dir.join("examples");
    std::fs::create_dir_all(&examples_dir)
        .map_err(|e| RustAiToolError::Io(e))?;
    
    let example_path = examples_dir.join("simple_example.rs");
    let example_content = format!(
        r#"fn main() {{
    println!("Example for {}: {{}}", {}::example_function());
}}
"#,
        config.name, config.name.replace('-', "_")
    );
    
    std::fs::write(&example_path, example_content)
        .map_err(|e| RustAiToolError::Io(e))?;
    
    Ok(())
}

/// Generate a CLI Rust project
///
/// # Arguments
///
/// * `project_dir` - Project directory
/// * `config` - Project configuration
///
/// # Returns
///
/// Success status
async fn generate_cli_project(project_dir: &Path, config: &ProjectConfig) -> Result<()> {
    // Create a basic project first
    generate_basic_project(project_dir, config).await?;
    
    // Create src/main.rs with CLI setup
    let main_rs_path = project_dir.join("src").join("main.rs");
    let main_rs_content = format!(
        r#"use clap::{{Parser, Subcommand}};

/// {} - {}
#[derive(Parser, Debug)]
#[clap(author, version, about)]
struct Cli {{
    /// Input file
    #[clap(short, long)]
    input: Option<std::path::PathBuf>,

    /// Output file
    #[clap(short, long)]
    output: Option<std::path::PathBuf>,

    /// Verbosity level
    #[clap(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Subcommand to execute
    #[clap(subcommand)]
    command: Option<Commands>,
}}

#[derive(Subcommand, Debug)]
enum Commands {{
    /// Example command
    Example {{
        /// Example argument
        #[clap(short, long)]
        name: String,
    }},
}}

fn main() {{
    let cli = Cli::parse();
    
    // Set up logging based on verbosity
    let log_level = match cli.verbose {{
        0 => log::LevelFilter::Warn,
        1 => log::LevelFilter::Info,
        2 => log::LevelFilter::Debug,
        _ => log::LevelFilter::Trace,
    }};
    
    env_logger::Builder::new()
        .filter_level(log_level)
        .init();
    
    log::info!("Starting application");
    
    // Handle subcommands
    match &cli.command {{
        Some(Commands::Example {{ name }}) => {{
            println!("Running example command with name: {{}}", name);
        }},
        None => {{
            println!("No subcommand specified, running default action");
        }},
    }}
}}
"#,
        config.name, config.description
    );
    
    std::fs::write(&main_rs_path, main_rs_content)
        .map_err(|e| RustAiToolError::Io(e))?;
    
    // Update Cargo.toml to add clap and logging dependencies if not already added
    let mut dependencies = vec!["clap".to_string(), "log".to_string(), "env_logger".to_string()];
    dependencies.retain(|d| !config.dependencies.contains(d));
    
    if !dependencies.is_empty() {
        let cargo_toml_path = project_dir.join("Cargo.toml");
        let cargo_toml = std::fs::read_to_string(&cargo_toml_path)
            .map_err(|e| RustAiToolError::Io(e))?;
        
        let mut cargo_doc = cargo_toml.parse::<toml::Document>()
            .map_err(|e| RustAiToolError::ProjectGeneration(format!("Failed to parse Cargo.toml: {}", e)))?;
        
        if let Some(deps) = cargo_doc.get_mut("dependencies") {
            if let Some(table) = deps.as_table_mut() {
                for dep in dependencies {
                    if dep == "clap" {
                        // Add clap with features
                        table.insert(
                            "clap",
                            toml::value::Value::Table({
                                let mut t = toml::Table::new();
                                t.insert(
                                    "version".to_string(),
                                    toml::value::Value::String("4.3".to_string()),
                                );
                                t.insert(
                                    "features".to_string(),
                                    toml::value::Value::Array(vec![
                                        toml::value::Value::String("derive".to_string()),
                                    ]),
                                );
                                t
                            }),
                        );
                    } else {
                        table.insert(dep, toml::value::Value::String("0.4".to_string()));
                    }
                }
            }
        }
        
        std::fs::write(&cargo_toml_path, cargo_doc.to_string())
            .map_err(|e| RustAiToolError::Io(e))?;
    }
    
    Ok(())
}

/// Generate a web service Rust project
///
/// # Arguments
///
/// * `project_dir` - Project directory
/// * `config` - Project configuration
///
/// # Returns
///
/// Success status
async fn generate_web_service_project(project_dir: &Path, config: &ProjectConfig) -> Result<()> {
    // Create a basic project first
    generate_basic_project(project_dir, config).await?;
    
    // Create src directory structure
    let src_dir = project_dir.join("src");
    std::fs::create_dir_all(&src_dir.join("routes"))
        .map_err(|e| RustAiToolError::Io(e))?;
    std::fs::create_dir_all(&src_dir.join("models"))
        .map_err(|e| RustAiToolError::Io(e))?;
    std::fs::create_dir_all(&src_dir.join("handlers"))
        .map_err(|e| RustAiToolError::Io(e))?;
    
    // Create main.rs with web server setup
    let main_rs_path = src_dir.join("main.rs");
    let main_rs_content = format!(
        r#"use actix_web::{{web, App, HttpServer, Responder, HttpResponse}};
use serde::{{Deserialize, Serialize}};

mod routes;
mod models;
mod handlers;

#[derive(Serialize)]
struct ApiResponse {{
    status: String,
    message: String,
}}

async fn health_check() -> impl Responder {{
    HttpResponse::Ok().json(ApiResponse {{
        status: "ok".to_string(),
        message: "Service is running".to_string(),
    }})
}}

#[actix_web::main]
async fn main() -> std::io::Result<()> {{
    // Initialize logger
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    
    log::info!("Starting {} server at http://localhost:8080", "{}");
    
    HttpServer::new(|| {{
        App::new()
            .route("/health", web::get().to(health_check))
            .configure(routes::init_routes)
    }})
    .bind("127.0.0.1:8080")?
    .run()
    .await
}}
"#,
        config.name, config.name
    );
    
    std::fs::write(&main_rs_path, main_rs_content)
        .map_err(|e| RustAiToolError::Io(e))?;
    
    // Create routes.rs
    let routes_rs_path = src_dir.join("routes.rs");
    let routes_rs_content = r#"use actix_web::web;
use crate::handlers;

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            .route("/example", web::get().to(handlers::get_example))
    );
}
"#;
    
    std::fs::write(&routes_rs_path, routes_rs_content)
        .map_err(|e| RustAiToolError::Io(e))?;
    
    // Create handlers.rs
    let handlers_rs_path = src_dir.join("handlers.rs");
    let handlers_rs_content = r#"use actix_web::{web, Responder, HttpResponse};
use serde::Serialize;

#[derive(Serialize)]
pub struct ExampleResponse {
    message: String,
    data: Vec<String>,
}

pub async fn get_example() -> impl Responder {
    let response = ExampleResponse {
        message: "Example endpoint".to_string(),
        data: vec!["item1".to_string(), "item2".to_string()],
    };
    
    HttpResponse::Ok().json(response)
}
"#;
    
    std::fs::write(&handlers_rs_path, handlers_rs_content)
        .map_err(|e| RustAiToolError::Io(e))?;
    
    // Create models.rs
    let models_rs_path = src_dir.join("models.rs");
    let models_rs_content = r#"use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ExampleModel {
    pub id: u32,
    pub name: String,
    pub active: bool,
}
"#;
    
    std::fs::write(&models_rs_path, models_rs_content)
        .map_err(|e| RustAiToolError::Io(e))?;
    
    // Update Cargo.toml to add web service dependencies
    let mut dependencies = vec![
        "actix-web".to_string(),
        "tokio".to_string(),
        "serde".to_string(),
        "serde_json".to_string(),
        "log".to_string(),
        "env_logger".to_string(),
    ];
    dependencies.retain(|d| !config.dependencies.contains(d));
    
    if !dependencies.is_empty() {
        let cargo_toml_path = project_dir.join("Cargo.toml");
        let cargo_toml = std::fs::read_to_string(&cargo_toml_path)
            .map_err(|e| RustAiToolError::Io(e))?;
        
        let mut cargo_doc = cargo_toml.parse::<toml::Document>()
            .map_err(|e| RustAiToolError::ProjectGeneration(format!("Failed to parse Cargo.toml: {}", e)))?;
        
        if let Some(deps) = cargo_doc.get_mut("dependencies") {
            if let Some(table) = deps.as_table_mut() {
                for dep in dependencies {
                    if dep == "tokio" {
                        // Add tokio with features
                        table.insert(
                            "tokio",
                            toml::value::Value::Table({
                                let mut t = toml::Table::new();
                                t.insert(
                                    "version".to_string(),
                                    toml::value::Value::String("1.28".to_string()),
                                );
                                t.insert(
                                    "features".to_string(),
                                    toml::value::Value::Array(vec![
                                        toml::value::Value::String("full".to_string()),
                                    ]),
                                );
                                t
                            }),
                        );
                    } else if dep == "serde" {
                        // Add serde with features
                        table.insert(
                            "serde",
                            toml::value::Value::Table({
                                let mut t = toml::Table::new();
                                t.insert(
                                    "version".to_string(),
                                    toml::value::Value::String("1.0".to_string()),
                                );
                                t.insert(
                                    "features".to_string(),
                                    toml::value::Value::Array(vec![
                                        toml::value::Value::String("derive".to_string()),
                                    ]),
                                );
                                t
                            }),
                        );
                    } else {
                        table.insert(dep, toml::value::Value::String("*".to_string()));
                    }
                }
            }
        }
        
        std::fs::write(&cargo_toml_path, cargo_doc.to_string())
            .map_err(|e| RustAiToolError::Io(e))?;
    }
    
    Ok(())
}

/// Generate a Tauri desktop application project
///
/// # Arguments
///
/// * `project_dir` - Project directory
/// * `config` - Project configuration
///
/// # Returns
///
/// Success status
async fn generate_tauri_project(project_dir: &Path, config: &ProjectConfig) -> Result<()> {
    // Create a basic project first
    generate_basic_project(project_dir, config).await?;
    
    // To initialize a Tauri project, we need to run Tauri CLI
    // This is complex and would involve Node.js setup as well
    // For simplicity, we'll just set up a skeleton
    
    // Create src-tauri directory
    let src_tauri_dir = project_dir.join("src-tauri");
    std::fs::create_dir_all(&src_tauri_dir.join("src"))
        .map_err(|e| RustAiToolError::Io(e))?;
    
    // Create main.rs for Tauri
    let main_rs_path = src_tauri_dir.join("src").join("main.rs");
    let main_rs_content = format!(
        r#"#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]

use tauri::{{Window, Manager}};
use serde::{{Deserialize, Serialize}};

#[derive(Debug, Serialize, Deserialize)]
struct ExampleResponse {{
  message: String,
  success: bool,
}}

#[tauri::command]
fn example_command(name: &str) -> ExampleResponse {{
  ExampleResponse {{
    message: format!("Hello, {{}}!", name),
    success: true,
  }}
}}

fn main() {{
  tauri::Builder::default()
    .setup(|app| {{
      // Initialize application
      #[cfg(debug_assertions)]
      {{
        let window = app.get_window("main").unwrap();
        window.open_devtools();
      }}
      Ok(())
    }})
    .invoke_handler(tauri::generate_handler![example_command])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}}
"#,
        config.name
    );
    
    std::fs::write(&main_rs_path, main_rs_content)
        .map_err(|e| RustAiToolError::Io(e))?;
    
    // Create tauri.conf.json
    let tauri_conf_path = src_tauri_dir.join("tauri.conf.json");
    let tauri_conf_content = format!(
        r#"{{
  "$schema": "https://tauri.app/v1/config-schema.json",
  "build": {{
    "beforeDevCommand": "",
    "beforeBuildCommand": "",
    "devPath": "../src",
    "distDir": "../src",
    "withGlobalTauri": true
  }},
  "package": {{
    "productName": "{}",
    "version": "0.1.0"
  }},
  "tauri": {{
    "allowlist": {{
      "all": false,
      "dialog": {{
        "all": true
      }},
      "fs": {{
        "all": true,
        "scope": ["$APP/*"]
      }}
    }},
    "bundle": {{
      "active": true,
      "icon": [
        "icons/32x32.png",
        "icons/128x128.png",
        "icons/128x128@2x.png",
        "icons/icon.icns",
        "icons/icon.ico"
      ],
      "identifier": "com.example.{}",
      "targets": "all"
    }},
    "security": {{
      "csp": null
    }},
    "windows": [
      {{
        "fullscreen": false,
        "resizable": true,
        "title": "{}",
        "width": 800,
        "height": 600
      }}
    ]
  }}
}}
"#,
        config.name,
        config.name.replace('-', ""),
        config.name
    );
    
    std::fs::write(&tauri_conf_path, tauri_conf_content)
        .map_err(|e| RustAiToolError::Io(e))?;
    
    // Create a simple frontend
    let src_dir = project_dir.join("src");
    std::fs::create_dir_all(&src_dir)
        .map_err(|e| RustAiToolError::Io(e))?;
    
    let index_html_path = src_dir.join("index.html");
    let index_html_content = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>{}</title>
  <style>
    body {{
      font-family: Arial, sans-serif;
      margin: 0;
      padding: 20px;
    }}
    h1 {{
      color: #333;
    }}
    button {{
      padding: 8px 16px;
      background-color: #4CAF50;
      color: white;
      border: none;
      border-radius: 4px;
      cursor: pointer;
    }}
    button:hover {{
      background-color: #45a049;
    }}
    #response {{
      margin-top: 20px;
      padding: 10px;
      border: 1px solid #ddd;
      border-radius: 4px;
      display: none;
    }}
  </style>
</head>
<body>
  <h1>{}</h1>
  <p>{}</p>
  
  <div>
    <input type="text" id="nameInput" placeholder="Enter your name">
    <button id="greetButton">Greet</button>
  </div>
  
  <div id="response"></div>

  <script>
    // Wait for Tauri API to be ready
    document.addEventListener('DOMContentLoaded', () => {{
      // We need to check if we're running in Tauri
      const isTauri = window.__TAURI__ !== undefined;
      
      const nameInput = document.getElementById('nameInput');
      const greetButton = document.getElementById('greetButton');
      const responseDiv = document.getElementById('response');
      
      greetButton.addEventListener('click', async () => {{
        const name = nameInput.value || 'World';
        
        if (isTauri) {{
          // Call Tauri command
          const response = await window.__TAURI__.invoke('example_command', {{ name }});
          responseDiv.textContent = response.message;
          responseDiv.style.display = 'block';
        }} else {{
          // Fallback for browser
          responseDiv.textContent = `Hello, ${{name}}! (Tauri API not available)`;
          responseDiv.style.display = 'block';
        }}
      }});
    }});
  </script>
</body>
</html>
"#,
        config.name,
        config.name,
        config.description
    );
    
    std::fs::write(&index_html_path, index_html_content)
        .map_err(|e| RustAiToolError::Io(e))?;
    
    // Create a README with Tauri-specific instructions
    let readme_path = project_dir.join("README.md");
    let readme_content = format!(
        r#"# {}

{}

## Prerequisites

- Rust
- Node.js
- Tauri CLI (`cargo install tauri-cli`)

## Development

```bash
# Install dependencies
cargo install tauri-cli

# Run in development mode
cargo tauri dev

# Build for production
cargo tauri build
```

## Features

- Cross-platform desktop application
- Rust backend with Tauri
- Simple HTML/CSS/JS frontend
"#,
        config.name, config.description
    );
    
    std::fs::write(&readme_path, readme_content)
        .map_err(|e| RustAiToolError::Io(e))?;
    
    // Update Cargo.toml to add Tauri dependencies
    let mut dependencies = vec![
        "tauri".to_string(),
        "serde".to_string(),
        "serde_json".to_string(),
    ];
    dependencies.retain(|d| !config.dependencies.contains(d));
    
    if !dependencies.is_empty() {
        let cargo_toml_path = project_dir.join("Cargo.toml");
        let cargo_toml = std::fs::read_to_string(&cargo_toml_path)
            .map_err(|e| RustAiToolError::Io(e))?;
        
        let mut cargo_doc = cargo_toml.parse::<toml::Document>()
            .map_err(|e| RustAiToolError::ProjectGeneration(format!("Failed to parse Cargo.toml: {}", e)))?;
        
        if let Some(deps) = cargo_doc.get_mut("dependencies") {
            if let Some(table) = deps.as_table_mut() {
                for dep in dependencies {
                    if dep == "tauri" {
                        // Add tauri with features
                        table.insert(
                            "tauri",
                            toml::value::Value::Table({
                                let mut t = toml::Table::new();
                                t.insert(
                                    "version".to_string(),
                                    toml::value::Value::String("1.4".to_string()),
                                );
                                t.insert(
                                    "features".to_string(),
                                    toml::value::Value::Array(vec![
                                        toml::value::Value::String("dialog".to_string()),
                                        toml::value::Value::String("fs".to_string()),
                                    ]),
                                );
                                t
                            }),
                        );
                    } else if dep == "serde" {
                        // Add serde with features
                        table.insert(
                            "serde",
                            toml::value::Value::Table({
                                let mut t = toml::Table::new();
                                t.insert(
                                    "version".to_string(),
                                    toml::value::Value::String("1.0".to_string()),
                                );
                                t.insert(
                                    "features".to_string(),
                                    toml::value::Value::Array(vec![
                                        toml::value::Value::String("derive".to_string()),
                                    ]),
                                );
                                t
                            }),
                        );
                    } else {
                        table.insert(dep, toml::value::Value::String("*".to_string()));
                    }
                }
            }
        }
        
        std::fs::write(&cargo_toml_path, cargo_doc.to_string())
            .map_err(|e| RustAiToolError::Io(e))?;
    }
    
    Ok(())
}

/// Generate a custom Rust project from a template
///
/// # Arguments
///
/// * `project_dir` - Project directory
/// * `config` - Project configuration
/// * `template_path` - Path to the template
///
/// # Returns
///
/// Success status
async fn generate_custom_project(project_dir: &Path, config: &ProjectConfig, template_path: &str) -> Result<()> {
    // Create a basic project first
    generate_basic_project(project_dir, config).await?;
    
    // In a real implementation, this would copy files from a template directory
    // and replace template variables with values from the configuration
    
    log::info!("Using custom template: {}", template_path);
    log::warn!("Custom template support is limited. Using basic project instead.");
    
    // For now, we'll just note that the user wanted to use a custom template
    let readme_path = project_dir.join("README.md");
    if readme_path.exists() {
        let content = std::fs::read_to_string(&readme_path)
            .map_err(|e| RustAiToolError::Io(e))?;
        
        let updated_content = format!(
            "{}\n\n> Note: This project was based on the custom template: {}\n",
            content, template_path
        );
        
        std::fs::write(&readme_path, updated_content)
            .map_err(|e| RustAiToolError::Io(e))?;
    }
    
    Ok(())
}

/// Initialize a Git repository
///
/// # Arguments
///
/// * `project_dir` - Project directory
///
/// # Returns
///
/// Success status
async fn init_git_repository(project_dir: &Path) -> Result<()> {
    // Initialize Git repository
    let init_result = Command::new("git")
        .arg("init")
        .current_dir(project_dir)
        .status()
        .await
        .map_err(|e| RustAiToolError::Io(e))?;
    
    if !init_result.success() {
        return Err(RustAiToolError::ProjectGeneration(format!(
            "Failed to initialize Git repository (exit code: {:?})",
            init_result.code()
        )));
    }
    
    // Add all files
    let add_result = Command::new("git")
        .args(&["add", "."])
        .current_dir(project_dir)
        .status()
        .await
        .map_err(|e| RustAiToolError::Io(e))?;
    
    if !add_result.success() {
        return Err(RustAiToolError::ProjectGeneration(format!(
            "Failed to add files to Git repository (exit code: {:?})",
            add_result.code()
        )));
    }
    
    // Commit
    let commit_result = Command::new("git")
        .args(&["commit", "-m", "Initial commit"])
        .current_dir(project_dir)
        .status()
        .await
        .map_err(|e| RustAiToolError::Io(e))?;
    
    if !commit_result.success() {
        log::warn!("Failed to create initial commit. This is not fatal, but you should commit your changes manually.");
    }
    
    Ok(())
}

/// Generate main.rs content using AI
///
/// # Arguments
///
/// * `project_dir` - Project directory
/// * `config` - Project configuration
/// * `ai_model` - AI model configuration
///
/// # Returns
///
/// Success status
async fn generate_main_rs_with_ai(
    project_dir: &Path,
    config: &ProjectConfig,
    ai_model: &AiModelConfig,
) -> Result<()> {
    log::info!("Generating main.rs content with AI...");
    
    // In a real implementation, this would call an AI API to generate code
    // For now, we'll just create a placeholder
    
    let main_rs_path = project_dir.join("src").join("main.rs");
    let main_rs_content = format!(
        r#"// This file was generated with AI assistance
// Project: {}
// Description: {}

fn main() {{
    println!("Hello from {}!");
    println!("Description: {{}}", env!("CARGO_PKG_DESCRIPTION"));
    
    run_example();
}}

fn run_example() {{
    println!("Running example functionality...");
    
    // Example code would be generated here based on the project description
    // For a more complete implementation, we would use the AI model to
    // generate appropriate starter code for the project
}}

#[cfg(test)]
mod tests {{
    use super::*;
    
    #[test]
    fn test_example() {{
        // Example test would be generated here
        assert!(true);
    }}
}}
"#,
        config.name,
        config.description,
        config.name
    );
    
    std::fs::write(&main_rs_path, main_rs_content)
        .map_err(|e| RustAiToolError::Io(e))?;
    
    Ok(())
}