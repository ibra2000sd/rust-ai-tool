use crate::{AnalysisOptions, Result, RustAiToolError, Severity, CustomRule};
use ra_ap_syntax::{SourceFile, SyntaxNode, TextRange, Parse};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use log::{debug, info, warn, error};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    pub file_path: PathBuf,
    pub issues: Vec<CodeIssue>,
    pub errors: Vec<String>,
    pub success: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeIssue {
    pub file_path: PathBuf,
    pub line_start: usize,
    pub column_start: usize,
    pub line_end: usize,
    pub column_end: usize,
    pub category: IssueCategory,
    pub severity: Severity,
    pub message: String,
    pub suggested_fix: Option<CodeFix>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum IssueCategory {
    Syntax,
    Semantic,
    Style,
    Performance,
    Security,
    TauriCompatibility,
    CodeQuality,
    CustomRule(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeFix {
    pub original_code: String,
    pub replacement_code: String,
    pub confidence: u8,
    pub description: String,
}

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

pub fn analyze_project(project_path: &Path, options: &AnalysisOptions) -> Result<Vec<AnalysisResult>> {
    info!("Analyzing Rust project at {}", project_path.display());
    
    let rust_files = collect_rust_files(project_path)?;
    debug!("Found {} Rust files to analyze", rust_files.len());
    
    let mut results = Vec::new();
    for file_path in rust_files {
        match analyze_file(&file_path, options) {
            Ok(result) => results.push(result),
            Err(e) => {
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
    
    if options.run_clippy {
        match run_clippy_project(project_path) {
            Ok(clippy_issues) => {
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

fn analyze_file(file_path: &Path, options: &AnalysisOptions) -> Result<AnalysisResult> {
    debug!("Analyzing file: {}", file_path.display());
    
    let file_content = std::fs::read_to_string(file_path)
        .map_err(|e| RustAiToolError::Io(e))?;
    
    let mut result = AnalysisResult {
        file_path: file_path.to_path_buf(),
        issues: Vec::new(),
        errors: Vec::new(),
        success: true,
    };
    
    if options.use_rust_analyzer {
        match analyze_syntax(&file_content, file_path) {
            Ok(syntax_issues) => result.issues.extend(syntax_issues),
            Err(e) => {
                result.errors.push(format!("Syntax analysis failed: {}", e));
                result.success = false;
            }
        }
    }
    
    for rule in &options.custom_rules {
        match apply_custom_rule(rule, &file_content, file_path) {
            Ok(rule_issues) => result.issues.extend(rule_issues),
            Err(e) => {
                result.errors.push(format!("Custom rule '{}' failed: {}", rule.name, e));
            }
        }
    }
    
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

fn analyze_syntax(content: &str, file_path: &Path) -> Result<Vec<CodeIssue>> {
    let mut issues = Vec::new();
    
    let parsed = SourceFile::parse(content);
    
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

fn find_syntax_errors(node: &SyntaxNode) -> Vec<TextRange> {
    let mut errors = Vec::new();
    
    for child in node.descendants() {
        if child.kind() == ra_ap_syntax::SyntaxKind::ERROR {
            errors.push(child.text_range());
        }
    }
    
    errors
}

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
                    if diagnostic.level == "warning" || diagnostic.level == "error" {
                        for span in diagnostic.spans.iter().filter(|s| s.is_primary) {
                            let file_path = PathBuf::from(&span.file_name);
                            
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

fn is_hidden(entry: &walkdir::DirEntry) -> bool {
    entry.file_name()
        .to_str()
        .map(|s| s.starts_with('.'))
        .unwrap_or(false)
}

fn is_target_dir(entry: &walkdir::DirEntry) -> bool {
    entry.file_name()
        .to_str()
        .map(|s| s == "target")
        .unwrap_or(false)
}

fn is_tauri_file(file_path: &Path) -> bool {
    let path_str = file_path.to_string_lossy();
    path_str.contains("src-tauri") || path_str.contains("tauri.conf.json")
}

fn analyze_tauri_compatibility(content: &str, file_path: &Path) -> Result<Vec<CodeIssue>> {
    let mut issues = Vec::new();
    
    let commands = extract_tauri_commands(content);
    let handlers = extract_invoke_handlers(content);
    
    for cmd in &commands {
        let is_registered = handlers.iter().any(|h| h.contains(cmd));
        
        if !is_registered {
            issues.push(CodeIssue {
                file_path: file_path.to_path_buf(),
                line_start: 0,
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

fn extract_tauri_commands(code: &str) -> Vec<String> {
    let command_pattern = r"#\[tauri::command\]\s*(?:pub\s+)?fn\s+([a-zA-Z0-9_]+)";
    
    let mut commands = Vec::new();
    
    if let Ok(re) = ::regex::Regex::new(command_pattern) {
        for cap in re.captures_iter(code) {
            if let Some(m) = cap.get(1) {
                commands.push(m.as_str().to_string());
            }
        }
    }
    
    commands
}

fn extract_invoke_handlers(code: &str) -> Vec<String> {
    let handler_pattern = r"\.invoke_handler\(([^)]+)\)";
    
    let mut handlers = Vec::new();
    
    if let Ok(re) = ::regex::Regex::new(handler_pattern) {
        for cap in re.captures_iter(code) {
            if let Some(m) = cap.get(1) {
                handlers.push(m.as_str().to_string());
            }
        }
    }
    
    handlers
}

fn apply_custom_rule(
    rule: &CustomRule,
    content: &str,
    file_path: &Path,
) -> Result<Vec<CodeIssue>> {
    let mut issues = Vec::new();
    
    if let Ok(re) = ::regex::Regex::new(&rule.pattern) {
        for cap in re.captures_iter(content) {
            if let Some(m) = cap.get(0) {
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
    } else {
        return Err(RustAiToolError::Analysis(format!("Invalid regex in custom rule '{}': {}", rule.name, rule.pattern)));
    }
    
    Ok(issues)
}