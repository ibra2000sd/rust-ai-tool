//! # Rust AI-Powered Project Analyzer & Code Refactoring Tool
//!
//! This library provides tools for analyzing, validating, and refactoring Rust code
//! with AI assistance. It integrates with Rust Analyzer, Clippy, and AI models to
//! provide intelligent code suggestions and automated fixes.

pub mod analysis;
pub mod validation;
pub mod project_generator;
pub mod modification;
pub mod cli;
pub mod github;
pub mod models;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors that can occur in the Rust AI Tool
#[derive(Error, Debug)]
pub enum RustAiToolError {
    /// Errors related to analysis of Rust code
    #[error("Analysis error: {0}")]
    Analysis(String),

    /// Errors related to validation of suggested fixes
    #[error("Validation error: {0}")]
    Validation(String),

    /// Errors related to project generation
    #[error("Project generation error: {0}")]
    ProjectGeneration(String),

    /// Errors related to code modification
    #[error("Code modification error: {0}")]
    Modification(String),

    /// Errors related to GitHub API interactions
    #[error("GitHub API error: {0}")]
    GitHub(String),

    /// Errors related to AI model integration
    #[error("AI model error: {0}")]
    AiModel(String),

    /// Errors related to file I/O
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization/deserialization errors
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Other errors
    #[error("Other error: {0}")]
    Other(String),
}

/// Result type for Rust AI Tool operations
pub type Result<T> = std::result::Result<T, RustAiToolError>;

/// Core configuration for the Rust AI Tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Path to the Rust project to analyze
    #[serde(skip)]
    pub project_path: std::path::PathBuf,
    
    /// GitHub repository information (if enabled)
    pub github_repo: Option<GitHubRepo>,
    
    /// AI model configuration
    pub ai_model: AiModelConfig,
    
    /// Analysis options
    pub analysis_options: AnalysisOptions,
    
    /// Validation options
    pub validation_options: ValidationOptions,
}

/// GitHub repository information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubRepo {
    /// GitHub repository owner
    pub owner: String,
    
    /// GitHub repository name
    pub name: String,
    
    /// GitHub access token
    pub access_token: String,
}

/// AI model configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiModelConfig {
    /// Type of AI model to use
    pub model_type: AiModelType,
    
    /// API key for accessing the AI model
    pub api_key: String,
    
    /// Base URL for the AI model API
    pub api_base_url: Option<String>,
}

/// Supported AI model types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AiModelType {
    /// Claude AI model
    Claude,
    
    /// GPT model from OpenAI
    Gpt,
    
    /// Mistral AI model
    Mistral,
    
    /// Local model (e.g., using Ollama)
    Local(String),
}

/// Options for code analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisOptions {
    /// Whether to run Clippy
    pub run_clippy: bool,
    
    /// Whether to analyze with Rust Analyzer
    pub use_rust_analyzer: bool,
    
    /// Custom rules to apply during analysis
    #[serde(default)]
    pub custom_rules: Vec<CustomRule>,
}

/// Options for validation of suggested fixes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationOptions {
    /// Whether to validate syntax only
    pub syntax_only: bool,
    
    /// Whether to validate against Tauri compatibility
    pub tauri_compatibility: bool,
    
    /// Whether to validate security implications
    pub security_validation: bool,
}

/// Custom analysis rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomRule {
    /// Name of the rule
    pub name: String,
    
    /// Pattern to match (regex or AST pattern)
    pub pattern: String,
    
    /// Message to display when the rule is triggered
    pub message: String,
    
    /// Severity of the rule
    pub severity: Severity,
}

/// Severity of an issue or rule
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Severity {
    /// Error - must be fixed
    Error,
    
    /// Warning - should be fixed
    Warning,
    
    /// Information - optional fix
    Info,
    
    /// Style - code style issue
    Style,
}