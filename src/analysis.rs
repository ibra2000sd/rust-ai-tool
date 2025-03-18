//! Code analysis module for detecting issues in Rust code
//!
//! This module provides functionality to analyze Rust code for various issues:
//! - Syntax and semantic errors
//! - Style and code quality issues (via Clippy)
//! - Security vulnerabilities
//! - Performance issues
//! - Tauri compatibility issues

use crate::{AnalysisOptions, Result, RustAiToolError, Severity, CustomRule};
use ra_ap_syntax::{AstNode, SourceFile, SyntaxNode, TextRange, TextSize, Parser};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use log::{debug, info, warn, error};

/// Represents the result of analyzing a Rust file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    /// Path to the analyzed file
    pub file_path: PathBuf,
    
    /// Issues found in the file
    pub issues: Vec<CodeIssue>,
    
    /// Errors encountered during analysis
    pub errors: Vec<String>,
    
    /// Whether the file was successfully analyzed
    pub success: bool,
}

/// Represents an issue found in Rust code
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeIssue {
    /// File where the issue was found
    pub file_path: PathBuf,
    
    /// Line number where the issue starts
    pub line_start: usize,
    
    /// Column number where the issue starts
    pub column_start: usize,
    
    /// Line number where the issue ends
    pub line_end: usize,
    
    /// Column number where the issue ends
    pub column_end: usize,
    
    /// The category of the issue
    pub category: IssueCategory,
    
    /// The severity of the issue
    pub severity: Severity,
    
    /// Description of the issue
    pub message: String,
    
    /// Suggested fix for the issue
    pub suggested_fix: Option<CodeFix>,
}

/// Categories of code issues
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum IssueCategory {
    /// Syntax error
    Syntax,
    
    /// Semantic error
    Semantic,
    
    /// Code style issue
    Style,
    
    /// Performance issue
    Performance,
    
    /// Security vulnerability
    Security,
    
    /// Tauri compatibility issue
    TauriCompatibility,
    
    /// General code quality issue
    CodeQuality,
    
    /// Custom rule violation
    CustomRule(String),
}

/// Represents a suggested fix for a code issue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeFix {
    /// Original code that should be replaced
    pub original_code: String,
    
    /// Replacement code
    pub replacement_code: String,
    
    /// Confidence level for this fix (0-100)
    pub confidence: u8,
    
    /// Description of the fix
    pub description: String,
}

/// Clippy message format for JSON output
#[derive(Debug, Deserialize)]
struct ClippyMessage {
    reason: String,
    message: Option<ClippyDiagnostic>,
}

#[derive(Debug, Deserialize)]
struct ClippyDiagnostic {
    code: Option<ClippyCode>,
    level: String,
    message: String,
    spans: Vec<ClippySpan>,
}

#[derive(Debug, Deserialize)]
struct ClippyCode {
    code: String,
}

#[derive(Debug, Deserialize)]
struct ClippySpan {
    file_name: String,
    line_start: u32,
    line_end: u32,
    column_start: u32,
    column_end: u32,
    is_primary: bool,
    text: Vec<ClippyText>,
}

#[derive(Debug, Deserialize)]
struct ClippyText {
    text: String,
}

/// Analyzes a Rust project for issues
///
/// # Arguments
///
/// * `project_path` - Path to the Rust project to analyze
/// * `options` - Analysis options
///
/// # Returns
///
/// A list of analysis results, one for each file in the project
pub fn analyze_project(project_path: &Path, options: &AnalysisOptions) -> Result<Vec<AnalysisResult>> {
    info!("Analyzing Rust project at {}", project_path.display());
    
    // Collect Rust files
    let rust_files = collect_rust_files(project_path)?;
    debug!("Found {} Rust files to analyze", rust_files.len());
    
    // Analyze each file
    let mut results = Vec::new();
    for file_path in rust_files {
        match analyze_file(&file_path, options) {
            Ok(result) => results.push(result),
            Err(e) => {
                // Log error but continue with other files
                error!("Failed to analyze file {}: {}", file_path.display(), e);
                results.push(AnalysisResult {
                    file_path,
                    issues: Vec::new(),
                    errors: vec![e.to_string()],
                    success: false,
                });
            }
        }
    }
    
    // If Clippy is enabled, run it once for the entire project
    if options.run_clippy {
        match run_clippy_project(project_path) {
            Ok(clippy_issues) => {
                // Group issues by file and add to results
                let issues_by_file = clippy_issues.iter()
                    .fold(HashMap::new(), |mut map, issue| {
                        map.entry(issue.file_path.clone())
                            .or_insert_with(Vec::new)
                            .push(issue.clone());
                        map
                    });
                
                for result in &mut results {
                    if let Some(file_issues) = issues_by_file.get(&result.file_path) {
                        result.issues.extend(file_issues.clone());
                    }
                }
            },
            Err(e) => {
                warn!("Failed to run Clippy on project: {}", e);
                for result in &mut results {
                    result.errors.push(format!("Clippy analysis failed: {}", e));
                }
            }
        }
    }
    
    Ok(results)
}

/// Analyzes a single Rust file for issues
///
/// # Arguments
///
/// * `file_path` - Path to the Rust file to analyze
/// * `options` - Analysis options
///
/// # Returns
///
/// Analysis result for the file
fn analyze_file(file_path: &Path, options: &AnalysisOptions) -> Result<AnalysisResult> {
    debug!("Analyzing file: {}", file_path.display());
    
    // Read file content
    let file_content = std::fs::read_to_string(file_path)
        .map_err(|e| RustAiToolError::Io(e))?;
    
    // Create a result object
    let mut result = AnalysisResult {
        file_path: file_path.to_path_buf(),
        issues: Vec::new(),
        errors: Vec::new(),
        success: true,
    };
    
    // Syntax analysis with ra_ap_syntax
    if options.use_rust_analyzer {
        match analyze_syntax(&file_content, file_path) {
            Ok(syntax_issues) => result.issues.extend(syntax_issues),
            Err(e) => {
                result.errors.push(format!("Syntax analysis failed: {}", e));
                result.success = false;
            }
        }
    }
    
    // Apply custom rules
    for rule in &options.custom_rules {
        match apply_custom_rule(rule, &file_content, file_path) {
            Ok(rule_issues) => result.issues.extend(rule_issues),
            Err(e) => {
                result.errors.push(format!("Custom rule '{}' failed: {}", rule.name, e));
            }
        }
    }
    
    // Check for Tauri-specific issues
    if is_tauri_file(file_path) {
        match analyze_tauri_compatibility(&file_content, file_path) {
            Ok(tauri_issues) => result.issues.extend(tauri_issues),
            Err(e) => {
                result.errors.push(format!("Tauri compatibility analysis failed: {}", e));
            }
        }
    }
    
    Ok(result)
}

/// Analyzes Rust code for syntax issues
///
/// # Arguments
///
/// * `content` - Rust code content
/// * `file_path` - Path to the file being analyzed
///
/// # Returns
///
/// List of syntax issues
fn analyze_syntax(content: &str, file_path: &Path) -> Result<Vec<CodeIssue>> {
    let mut issues = Vec::new();
    
    // Parse the file with ra_ap_syntax
    let parsed = SourceFile::parse(content);
    
    // Extract syntax errors
    for error in find_syntax_errors(&parsed.syntax_node()) {
        let (line_start, column_start) = offset_to_line_column(content, error.start().into());
        let (line_end, column_end) = offset_to_line_column(content, error.end().into());
        
        issues.push(CodeIssue {
            file_path: file_path.to_path_buf(),
            line_start,
            column_start,
            line_end,
            column_end,
            category: IssueCategory::Syntax,
            severity: Severity::Error,
            message: "Syntax error".to_string(),
            suggested_fix: None,
        });
    }
    
    Ok(issues)
}

/// Find syntax errors in a syntax node
fn find_syntax_errors(node: &SyntaxNode) -> Vec<TextRange> {
    let mut errors = Vec::new();
    
    for child in node.descendants() {
        if child.kind() == ra_ap_syntax::SyntaxKind::ERROR {
            errors.push(child.text_range());
        }
    }
    
    errors
}

/// Convert byte offset to line and column
fn offset_to_line_column(text: &str, offset: usize) -> (usize, usize) {
    let mut line = 1;
    let mut col = 1;
    
    for (i, c) in text.char_indices() {
        if i >= offset {
            break;
        }
        
        if c == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    
    (line, col)
}

/// Runs Clippy on an entire project
///
/// # Arguments
///
/// * `project_path` - Path to the Rust project
///
/// # Returns
///
/// List of issues found by Clippy
fn run_clippy_project(project_path: &Path) -> Result<Vec<CodeIssue>> {
    debug!("Running Clippy on project at {}", project_path.display());
    
    let output = Command::new("cargo")
        .args(&["clippy", "--message-format=json", "--", "-W", "clippy::all"])
        .current_dir(project_path)
        .output()
        .map_err(|e| RustAiToolError::Analysis(format!("Failed to execute Clippy: {}", e)))?;
    
    if !output.status.success() {
        warn!("Clippy exited with non-zero status: {}", output.status);
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut issues = Vec::new();
    
    for line in stdout.lines() {
        if let Ok(message) = serde_json::from_str::<ClippyMessage>(line) {
            if message.reason == "compiler-message" {
                if let Some(diagnostic) = message.message {
                    // Process only warnings and errors
                    if diagnostic.level == "warning" || diagnostic.level == "error" {
                        // Find the primary span
                        for span in diagnostic.spans.iter().filter(|s| s.is_primary) {
                            let file_path = PathBuf::from(&span.file_name);
                            
                            // Skip if not a real file (like <macro>)
                            if !file_path.exists() {
                                continue;
                            }
                            
                            let severity = match diagnostic.level.as_str() {
                                "error" => Severity::Error,
                                "warning" => Severity::Warning,
                                _ => Severity::Info,
                            };
                            
                            issues.push(CodeIssue {
                                file_path,
                                line_start: span.line_start as usize,
                                column_start: span.column_start as usize,
                                line_end: span.line_end as usize,
                                column_end: span.column_end as usize,
                                category: IssueCategory::CodeQuality,
                                severity,
                                message: diagnostic.message.clone(),
                                suggested_fix: None,
                            });
                        }
                    }
                }
            }
        }
    }
    
    Ok(issues)
}

/// Collects all Rust files in a project
fn collect_rust_files(project_path: &Path) -> Result<Vec<PathBuf>> {
    let mut rust_files = Vec::new();
    
    let walker = walkdir::WalkDir::new(project_path)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| !is_hidden(e) && !is_target_dir(e));
    
    for entry in walker.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_file() && path.extension().map_or(false, |ext| ext == "rs") {
            rust_files.push(path.to_path_buf());
        }
    }
    
    Ok(rust_files)
}

/// Checks if a directory entry is hidden
fn is_hidden(entry: &walkdir::DirEntry) -> bool {
    entry.file_name()
        .to_str()
        .map(|s| s.starts_with('.'))
        .unwrap_or(false)
}

/// Checks if a directory entry is a target directory
fn is_target_dir(entry: &walkdir::DirEntry) -> bool {
    entry.file_name()
        .to_str()
        .map(|s| s == "target")
        .unwrap_or(false)
}

/// Checks if a file is part of a Tauri project
fn is_tauri_file(file_path: &Path) -> bool {
    // Check if the file is in a src-tauri directory
    let path_str = file_path.to_string_lossy();
    path_str.contains("src-tauri") || path_str.contains("tauri.conf.json")
}

/// Analyze Tauri-specific issues
fn analyze_tauri_compatibility(content: &str, file_path: &Path) -> Result<Vec<CodeIssue>> {
    let mut issues = Vec::new();
    
    // Extract Tauri commands
    let commands = extract_tauri_commands(content);
    
    // Extract invoke handlers
    let handlers = extract_invoke_handlers(content);
    
    // Check if all commands are registered in handlers
    for cmd in &commands {
        let is_registered = handlers.iter().any(|h| h.contains(cmd));
        
        if !is_registered {
            issues.push(CodeIssue {
                file_path: file_path.to_path_buf(),
                line_start: 0, // We'll need to find the actual line
                column_start: 0,
                line_end: 0,
                column_end: 0,
                category: IssueCategory::TauriCompatibility,
                severity: Severity::Error,
                message: format!("Tauri command '{}' is not registered in any invoke_handler", cmd),
                suggested_fix: None,
            });
        }
    }
    
    Ok(issues)
}

/// Extracts Tauri commands from code
fn extract_tauri_commands(code: &str) -> Vec<String> {
    let command_regex = regex::Regex::new(r"#\[tauri::command\]\s*(?:pub\s+)?fn\s+([a-zA-Z0-9_]+)").unwrap();
    
    command_regex
        .captures_iter(code)
        .map(|cap| cap[1].to_string())
        .collect()
}

/// Extracts Tauri invoke handlers from code
fn extract_invoke_handlers(code: &str) -> Vec<String> {
    let handler_regex = regex::Regex::new(r"\.invoke_handler\(([^)]+)\)").unwrap();
    
    handler_regex
        .captures_iter(code)
        .map(|cap| cap[1].to_string())
        .collect()
}

/// Apply a custom rule to a Rust file
fn apply_custom_rule(
    rule: &CustomRule,
    content: &str,
    file_path: &Path,
) -> Result<Vec<CodeIssue>> {
    let mut issues = Vec::new();
    
    // Use regex to match the pattern
    let re = regex::Regex::new(&rule.pattern)
        .map_err(|e| RustAiToolError::Analysis(format!("Invalid regex in custom rule '{}': {}", rule.name, e)))?;
    
    for capture in re.captures_iter(content) {
        if let Some(m) = capture.get(0) {
            let (line_start, column_start) = offset_to_line_column(content, m.start());
            let (line_end, column_end) = offset_to_line_column(content, m.end());
            
            issues.push(CodeIssue {
                file_path: file_path.to_path_buf(),
                line_start,
                column_start,
                line_end,
                column_end,
                category: IssueCategory::CustomRule(rule.name.clone()),
                severity: rule.severity.clone(),
                message: rule.message.clone(),
                suggested_fix: None,
            });
        }
    }
    
    Ok(issues)
}