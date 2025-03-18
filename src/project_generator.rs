//! Project generation module
//!
//! This module provides functionality to generate Rust projects:
//! - Create new Rust projects from templates
//! - Generate code based on AI descriptions
//! - Create project scaffolding with best practices

use crate::{Result, RustAiToolError, AiModelConfig};
use std::path::{Path, PathBuf};
use std::fs;
use tokio::process::Command;
use log::{debug, info, warn, error};
use serde::{Serialize, Deserialize};

/// Project template
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    
    /// Web service with Axum
    Axum,
    
    /// REST API with Rocket
    RocketApi,
    
    /// WebAssembly project
    WasmProject,
    
    /// Embedded Rust project
    EmbeddedRust,
    
    /// Machine Learning project
    MachineLearning,
    
    /// Custom template
    Custom(String),
}

/// Configuration for project generation
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    #[serde(skip)]
    pub output_dir: PathBuf,
    
    /// Whether to initialize a Git repository
    pub init_git: bool,
    
    /// Additional dependencies to include
    pub dependencies: Vec<String>,
    
    /// AI model configuration for code generation
    #[serde(skip)]
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
    info!("Generating project from description: {}", description);
    
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
    // Choose the appropriate template based on the description
    let template = determine_template(description);
    debug!("Selected template: {:?}", template);
    
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

/// Determine the best template for the project based on description
fn determine_template(description: &str) -> ProjectTemplate {
    let description = description.to_lowercase();
    
    // Check for specific keywords to select the appropriate template
    if description.contains("wasm") || description.contains("webassembly") {
        ProjectTemplate::WasmProject
    } else if description.contains("embedded") || description.contains("microcontroller") || description.contains("arduino") {
        ProjectTemplate::EmbeddedRust
    } else if description.contains("machine learning") || description.contains("ml") || description.contains("ai") {
        ProjectTemplate::MachineLearning
    } else if description.contains("tauri") || description.contains("desktop app") || description.contains("gui") {
        ProjectTemplate::TauriApp
    } else if description.contains("axum") {
        ProjectTemplate::Axum
    } else if description.contains("rocket") || description.contains("rest api") {
        ProjectTemplate::RocketApi
    } else if description.contains("web") || description.contains("server") || description.contains("api") {
        ProjectTemplate::WebService
    } else if description.contains("cli") || description.contains("command") {
        ProjectTemplate::Cli
    } else if description.contains("library") || description.contains("lib") {
        ProjectTemplate::Library
    } else {
        ProjectTemplate::Basic
    }
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
        "axum", "wasm-bindgen", "web-sys", "js-sys", "linfa",
        "embedded-hal", "cortex-m", "no_std", "alloc", "async-std",
    ];
    
    for crate_name in &known_crates {
        if description.to_lowercase().contains(crate_name) {
            dependencies.push(crate_name.to_string());
        }
    }
    
    // Add template-specific dependencies
    let template = determine_template(description);
    match template {
        ProjectTemplate::Cli => {
            if !dependencies.contains(&"clap".to_string()) {
                dependencies.push("clap".to_string());
            }
        },
        ProjectTemplate::WebService => {
            if !dependencies.contains(&"actix-web".to_string()) {
                dependencies.push("actix-web".to_string());
            }
        },
        ProjectTemplate::Axum => {
            if !dependencies.contains(&"axum".to_string()) {
                dependencies.push("axum".to_string());
            }
        },
        ProjectTemplate::RocketApi => {
            if !dependencies.contains(&"rocket".to_string()) {
                dependencies.push("rocket".to_string());
            }
        },
        ProjectTemplate::TauriApp => {
            if !dependencies.contains(&"tauri".to_string()) {
                dependencies.push("tauri".to_string());
            }
        },
        ProjectTemplate::WasmProject => {
            if !dependencies.contains(&"wasm-bindgen".to_string()) {
                dependencies.push("wasm-bindgen".to_string());
            }
        },
        ProjectTemplate::EmbeddedRust => {
            if !dependencies.contains(&"embedded-hal".to_string()) {
                dependencies.push("embedded-hal".to_string());
            }
        },
        ProjectTemplate::MachineLearning => {
            if !dependencies.contains(&"linfa".to_string()) {
                dependencies.push("linfa".to_string());
            }
        },
        _ => {}
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
    fs::create_dir_all(&project_dir)
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
    let cargo_toml = fs::read_to_string(&cargo_toml_path)
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
                // Handle special cases for specific dependencies
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
                } else if dep == "tokio" {
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
                } else if dep == "tauri" {
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
                } else {
                    // Default for other dependencies
                    table.insert(dep, toml::value::Value::String("*".to_string()));
                }
            }
        }
    }
    
    // Write the updated Cargo.toml
    fs::write(&cargo_toml_path, cargo_doc.to_string())
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
        "wasm", "ai", "ml", "embedded", "async", "blockchain"
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
        ProjectTemplate::Axum => generate_axum_project(project_dir, config).await?,
        ProjectTemplate::RocketApi => generate_rocket_project(project_dir, config).await?,
        ProjectTemplate::TauriApp => generate_tauri_project(project_dir, config).await?,
        ProjectTemplate::WasmProject => generate_wasm_project(project_dir, config).await?,
        ProjectTemplate::EmbeddedRust => generate_embedded_project(project_dir, config).await?,
        ProjectTemplate::MachineLearning => generate_ml_project(project_dir, config).await?,
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
    
    fs::write(&readme_path, readme_content)
        .map_err(|e| RustAiToolError::Io(e))?;
    
    // Create a .gitignore
    let gitignore_path = project_dir.join(".gitignore");
    let gitignore_content = r#"/target
**/*.rs.bk
Cargo.lock
"#;
    
    fs::write(&gitignore_path, gitignore_content)
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
//! // Example usage
//! ```
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
    
    fs::write(&lib_rs_path, lib_rs_content)
        .map_err(|e| RustAiToolError::Io(e))?;
    
    // Create examples directory with a simple example
    let examples_dir = project_dir.join("examples");
    fs::create_dir_all(&examples_dir)
        .map_err(|e| RustAiToolError::Io(e))?;
    
    let example_path = examples_dir.join("simple_example.rs");
    let example_content = format!(
        r#"fn main() {{
    println!("Example for {}: {{}}", {}::example_function());
}}
"#,
        config.name, config.name.replace('-', "_")
    );
    
    fs::write(&example_path, example_content)
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
    
    fs::write(&main_rs_path, main_rs_content)
        .map_err(|e| RustAiToolError::Io(e))?;
    
    // Update Cargo.toml to add clap and logging dependencies if not already added
    let mut dependencies = vec!["clap".to_string(), "log".to_string(), "env_logger".to_string()];
    dependencies.retain(|d| !config.dependencies.contains(d));
    
    if !dependencies.is_empty() {
        let cargo_toml_path = project_dir.join("Cargo.toml");
        let cargo_toml = fs::read_to_string(&cargo_toml_path)
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
        
        fs::write(&cargo_toml_path, cargo_doc.to_string())
            .map_err(|e| RustAiToolError::Io(e))?;
    }
    
    Ok(())
}

/// Generate a web service Rust project with Actix
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
    fs::create_dir_all(&src_dir.join("routes"))
        .map_err(|e| RustAiToolError::Io(e))?;
    fs::create_dir_all(&src_dir.join("models"))
        .map_err(|e| RustAiToolError::Io(e))?;
    fs::create_dir_all(&src_dir.join("handlers"))
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
    
    fs::write(&main_rs_path, main_rs_content)
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
    
    fs::write(&routes_rs_path, routes_rs_content)
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
    
    fs::write(&handlers_rs_path, handlers_rs_content)
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
    
    fs::write(&models_rs_path, models_rs_content)
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
        let cargo_toml = fs::read_to_string(&cargo_toml_path)
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
        
        fs::write(&cargo_toml_path, cargo_doc.to_string())
            .map_err(|e| RustAiToolError::Io(e))?;
    }
    
    Ok(())
}

/// Generate a web service Rust project with Axum
///
/// # Arguments
///
/// * `project_dir` - Project directory
/// * `config` - Project configuration
///
/// # Returns
///
/// Success status
async fn generate_axum_project(project_dir: &Path, config: &ProjectConfig) -> Result<()> {
    // Create a basic project first
    generate_basic_project(project_dir, config).await?;
    
    // Create src directory structure
    let src_dir = project_dir.join("src");
    fs::create_dir_all(&src_dir.join("routes"))
        .map_err(|e| RustAiToolError::Io(e))?;
    fs::create_dir_all(&src_dir.join("models"))
        .map_err(|e| RustAiToolError::Io(e))?;
    fs::create_dir_all(&src_dir.join("handlers"))
        .map_err(|e| RustAiToolError::Io(e))?;
    
    // Create main.rs with Axum setup
    let main_rs_path = src_dir.join("main.rs");
    let main_rs_content = format!(
        r#"use axum::{{
    extract::Extension,
    routing::{{get, post}},
    Router,
}};
use serde::{{Deserialize, Serialize}};
use std::net::SocketAddr;

mod routes;
mod models;
mod handlers;

#[tokio::main]
async fn main() {{
    // Initialize logger
    tracing_subscriber::fmt::init();
    
    // Build our application
    let app = Router::new()
        .route("/health", get(health_check))
        .nest("/api", routes::api_routes());
    
    // Run it
    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    tracing::info!("Starting {} server at http://localhost:8080", "{}");
    
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}}

#[derive(Serialize)]
struct HealthResponse {{
    status: String,
    message: String,
}}

// Basic health check handler
async fn health_check() -> axum::Json<HealthResponse> {{
    axum::Json(HealthResponse {{
        status: "ok".to_string(),
        message: "Service is running".to_string(),
    }})
}}
"#,
        config.name, config.name
    );
    
    fs::write(&main_rs_path, main_rs_content)
        .map_err(|e| RustAiToolError::Io(e))?;
    
    // Create routes.rs
    let routes_rs_path = src_dir.join("routes.rs");
    let routes_rs_content = r#"use axum::{
    routing::{get, post},
    Router,
};
use crate::handlers;

pub fn api_routes() -> Router {
    Router::new()
        .route("/example", get(handlers::get_example))
}
"#;
    
    fs::write(&routes_rs_path, routes_rs_content)
        .map_err(|e| RustAiToolError::Io(e))?;
    
    // Create handlers.rs
    let handlers_rs_path = src_dir.join("handlers.rs");
    let handlers_rs_content = r#"use axum::Json;
use serde::Serialize;

#[derive(Serialize)]
pub struct ExampleResponse {
    message: String,
    data: Vec<String>,
}

pub async fn get_example() -> Json<ExampleResponse> {
    Json(ExampleResponse {
        message: "Example endpoint".to_string(),
        data: vec!["item1".to_string(), "item2".to_string()],
    })
}
"#;
    
    fs::write(&handlers_rs_path, handlers_rs_content)
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
    
    fs::write(&models_rs_path, models_rs_content)
        .map_err(|e| RustAiToolError::Io(e))?;
    
    // Update Cargo.toml to add Axum dependencies
    let mut dependencies = vec![
        "axum".to_string(),
        "tokio".to_string(),
        "serde".to_string(),
        "serde_json".to_string(),
        "tracing".to_string(),
        "tracing-subscriber".to_string(),
    ];
    dependencies.retain(|d| !config.dependencies.contains(d));
    
    if !dependencies.is_empty() {
        let cargo_toml_path = project_dir.join("Cargo.toml");
        let cargo_toml = fs::read_to_string(&cargo_toml_path)
            .map_err(|e| RustAiToolError::Io(e))?;
        
        let mut cargo_doc = cargo_toml.parse::<toml::Document>()
            .map_err(|e| RustAiToolError::ProjectGeneration(format!("Failed to parse Cargo.toml: {}", e)))?;
        
        if let Some(deps) = cargo_doc.get_mut("dependencies") {
            if let Some(table) = deps.as_table_mut() {
                for dep in dependencies {
                    if dep == "tokio" {
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
                                        toml::value::Value::String("rt-multi-thread".to_string()),
                                    ]),
                                );
                                t
                            }),
                        );
                    } else if dep == "serde" {
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
        
        fs::write(&cargo_toml_path, cargo_doc.to_string())
            .map_err(|e| RustAiToolError::Io(e))?;
    }
    
    Ok(())
}

/// Generate a Rocket web API project
///
/// # Arguments
///
/// * `project_dir` - Project directory
/// * `config` - Project configuration
///
/// # Returns
///
/// Success status
async fn generate_rocket_project(project_dir: &Path, config: &ProjectConfig) -> Result<()> {
    // Create a basic project first
    generate_basic_project(project_dir, config).await?;
    
    // Create src directory structure
    let src_dir = project_dir.join("src");
    fs::create_dir_all(&src_dir.join("routes"))
        .map_err(|e| RustAiToolError::Io(e))?;
    fs::create_dir_all(&src_dir.join("models"))
        .map_err(|e| RustAiToolError::Io(e))?;
    
    // Create main.rs with Rocket setup
    let main_rs_path = src_dir.join("main.rs");
    let main_rs_content = format!(
        r#"#[macro_use] extern crate rocket;
use rocket::serde::{{Serialize, json::Json}};

mod routes;
mod models;

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct HealthResponse {{
    status: String,
    message: String,
}}

#[get("/health")]
fn health_check() -> Json<HealthResponse> {{
    Json(HealthResponse {{
        status: "ok".to_string(),
        message: "Service is running".to_string(),
    }})
}}

#[launch]
fn rocket() -> _ {{
    println!("Starting {} server", "{}");
    
    rocket::build()
        .mount("/", routes![health_check])
        .mount("/api", routes::routes())
}}
"#,
        config.name, config.name
    );
    
    fs::write(&main_rs_path, main_rs_content)
        .map_err(|e| RustAiToolError::Io(e))?;
    
    // Create routes.rs
    let routes_rs_path = src_dir.join("routes.rs");
    let routes_rs_content = r#"use rocket::{serde::json::Json, Route};
use crate::models::ExampleModel;

#[get("/example")]
fn example() -> Json<ExampleModel> {
    Json(ExampleModel {
        id: 1,
        name: "Example".to_string(),
        active: true,
    })
}

pub fn routes() -> Vec<Route> {
    routes![example]
}
"#;
    
    fs::write(&routes_rs_path, routes_rs_content)
        .map_err(|e| RustAiToolError::Io(e))?;
    
    // Create models.rs
    let models_rs_path = src_dir.join("models.rs");
    let models_rs_content = r#"use rocket::serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
pub struct ExampleModel {
    pub id: u32,
    pub name: String,
    pub active: bool,
}
"#;
    
    fs::write(&models_rs_path, models_rs_content)
        .map_err(|e| RustAiToolError::Io(e))?;
    
    // Update Cargo.toml to add Rocket dependencies
    let mut dependencies = vec![
        "rocket".to_string(),
    ];
    dependencies.retain(|d| !config.dependencies.contains(d));
    
    if !dependencies.is_empty() {
        let cargo_toml_path = project_dir.join("Cargo.toml");
        let cargo_toml = fs::read_to_string(&cargo_toml_path)
            .map_err(|e| RustAiToolError::Io(e))?;
        
        let mut cargo_doc = cargo_toml.parse::<toml::Document>()
            .map_err(|e| RustAiToolError::ProjectGeneration(format!("Failed to parse Cargo.toml: {}", e)))?;
        
        if let Some(deps) = cargo_doc.get_mut("dependencies") {
            if let Some(table) = deps.as_table_mut() {
                for dep in dependencies {
                    if dep == "rocket" {
                        table.insert(
                            "rocket",
                            toml::value::Value::Table({
                                let mut t = toml::Table::new();
                                t.insert(
                                    "version".to_string(),
                                    toml::value::Value::String("0.5.0".to_string()),
                                );
                                t.insert(
                                    "features".to_string(),
                                    toml::value::Value::Array(vec![
                                        toml::value::Value::String("json".to_string()),
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
        
        fs::write(&cargo_toml_path, cargo_doc.to_string())
            .map_err(|e| RustAiToolError::Io(e))?;
    }
    
    Ok(())
}