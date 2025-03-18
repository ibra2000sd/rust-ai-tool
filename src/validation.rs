//! Validation module for suggested code fixes
//!
//! This module provides functionality to validate suggested code fixes:
//! - Syntax validation
//! - Semantic validation
//! - Security implications
//! - Tauri compatibility
//! - Structural integrity

use crate::{RustAiToolError, ValidationOptions, Result};
use ra_ap_syntax::{SourceFile, SyntaxNode, SyntaxKind};
use std::path::{Path, PathBuf};
use serde::{Serialize, Deserialize};
use log::{debug, info, warn, error};

/// Result of validating a fix
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Path to the file being validated
    pub file_path: PathBuf,
    
    /// Whether the validation was successful
    pub is_valid: bool,
    
    /// Validation messages (errors or warnings)
    pub messages: Vec<ValidationMessage>,
    
    /// Severity of validation issues
    pub severity: ValidationSeverity,
}

/// Message from validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationMessage {
    /// Type of validation message
    pub message_type: ValidationMessageType,
    
    /// The message text
    pub text: String,
    
    /// Location in the code (if relevant)
    pub location: Option<CodeLocation>,
}

/// Location in code
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeLocation {
    /// Line number
    pub line: usize,
    
    /// Column number
    pub column: usize,
}

/// Types of validation messages
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ValidationMessageType {
    /// Error that prevents applying the fix
    Error,
    
    /// Warning that might indicate a problem
    Warning,
    
    /// Informational message
    Info,
}

impl std::fmt::Display for ValidationMessageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationMessageType::Error => write!(f, "ERROR"),
            ValidationMessageType::Warning => write!(f, "WARNING"),
            ValidationMessageType::Info => write!(f, "INFO"),
        }
    }
}

/// Severity of validation issues
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ValidationSeverity {
    /// Critical issue - must not apply the fix
    Critical,
    
    /// Major issue - should not apply the fix
    Major,
    
    /// Minor issue - can apply the fix with caution
    Minor,
    
    /// No issues found
    None,
}

impl ValidationSeverity {
    /// Convert to a boolean for the is_valid field
    pub fn is_valid(&self) -> bool {
        match self {
            ValidationSeverity::Critical => false,
            ValidationSeverity::Major => false,
            ValidationSeverity::Minor => true,
            ValidationSeverity::None => true,
        }
    }
}

/// Represents a code fix to validate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixToValidate {
    /// Path to the file to modify
    pub file_path: PathBuf,
    
    /// Original code
    pub original_code: String,
    
    /// Modified code with the fix applied
    pub modified_code: String,
    
    /// Description of the fix
    pub description: String,
}

/// Helper struct for partial validation results
#[derive(Debug)]
pub struct ValidationPartialResult {
    pub messages: Vec<ValidationMessage>,
    pub severity: ValidationSeverity,
}

impl ValidationPartialResult {
    /// Create a new empty validation result
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            severity: ValidationSeverity::None,
        }
    }
    
    /// Add a message to the result
    pub fn add_message(&mut self, message_type: ValidationMessageType, text: String, location: Option<CodeLocation>) {
        self.messages.push(ValidationMessage {
            message_type,
            text,
            location,
        });
    }
    
    /// Add an error message
    pub fn add_error(&mut self, text: String, location: Option<CodeLocation>) {
        self.add_message(ValidationMessageType::Error, text, location);
        if self.severity != ValidationSeverity::Critical {
            self.severity = ValidationSeverity::Major;
        }
    }
    
    /// Add a critical error message
    pub fn add_critical_error(&mut self, text: String, location: Option<CodeLocation>) {
        self.add_message(ValidationMessageType::Error, text, location);
        self.severity = ValidationSeverity::Critical;
    }
    
    /// Add a warning message
    pub fn add_warning(&mut self, text: String, location: Option<CodeLocation>) {
        self.add_message(ValidationMessageType::Warning, text, location);
        if self.severity == ValidationSeverity::None {
            self.severity = ValidationSeverity::Minor;
        }
    }
    
    /// Add an info message
    pub fn add_info(&mut self, text: String, location: Option<CodeLocation>) {
        self.add_message(ValidationMessageType::Info, text, location);
    }
}

/// Validates a list of suggested fixes
///
/// # Arguments
///
/// * `fixes` - List of fixes to validate
/// * `options` - Validation options
///
/// # Returns
///
/// A list of validation results, one for each fix
pub fn validate_fixes(fixes: &[FixToValidate], options: &ValidationOptions) -> Result<Vec<ValidationResult>> {
    info!("Validating {} fixes", fixes.len());
    let mut results = Vec::new();
    
    for (i, fix) in fixes.iter().enumerate() {
        debug!("Validating fix #{} for {}", i + 1, fix.file_path.display());
        match validate_fix(fix, options) {
            Ok(result) => {
                if result.is_valid {
                    debug!("Fix #{} is valid", i + 1);
                } else {
                    warn!("Fix #{} is invalid: {:?}", i + 1, result.severity);
                }
                results.push(result);
            },
            Err(e) => {
                // Log error but continue with other fixes
                error!("Failed to validate fix for {}: {}", fix.file_path.display(), e);
                results.push(ValidationResult {
                    file_path: fix.file_path.clone(),
                    is_valid: false,
                    messages: vec![ValidationMessage {
                        message_type: ValidationMessageType::Error,
                        text: format!("Validation error: {}", e),
                        location: None,
                    }],
                    severity: ValidationSeverity::Critical,
                });
            }
        }
    }
    
    Ok(results)
}

/// Validates a single fix
///
/// # Arguments
///
/// * `fix` - Fix to validate
/// * `options` - Validation options
///
/// # Returns
///
/// Validation result for the fix
pub fn validate_fix(fix: &FixToValidate, options: &ValidationOptions) -> Result<ValidationResult> {
    let mut messages = Vec::new();
    let mut severity = ValidationSeverity::None;
    
    // Always validate syntax
    let syntax_result = validate_syntax(&fix.modified_code);
    messages.extend(syntax_result.messages);
    
    // Update severity based on syntax validation
    if syntax_result.severity > severity {
        severity = syntax_result.severity;
    }
    
    // Check if we need to go beyond syntax validation
    if !options.syntax_only {
        // Validate semantic correctness
        let semantic_result = validate_semantics(&fix.file_path, &fix.modified_code);
        messages.extend(semantic_result.messages);
        
        // Update severity based on semantic validation
        if semantic_result.severity > severity {
            severity = semantic_result.severity;
        }
        
        // Validate structural integrity
        let structural_result = validate_structural_integrity(&fix.original_code, &fix.modified_code);
        messages.extend(structural_result.messages);
        
        // Update severity based on structural validation
        if structural_result.severity > severity {
            severity = structural_result.severity;
        }
        
        // Validate Tauri compatibility if needed
        if options.tauri_compatibility && is_tauri_file(&fix.file_path) {
            let tauri_result = validate_tauri_compatibility(&fix.original_code, &fix.modified_code);
            messages.extend(tauri_result.messages);
            
            // Update severity based on Tauri validation
            if tauri_result.severity > severity {
                severity = tauri_result.severity;
            }
        }
        
        // Validate security implications if needed
        if options.security_validation {
            let security_result = validate_security_implications(&fix.original_code, &fix.modified_code);
            messages.extend(security_result.messages);
            
            // Update severity based on security validation
            if security_result.severity > severity {
                severity = security_result.severity;
            }
        }
    }
    
    // A fix is valid if there are no critical or major issues
    let is_valid = severity != ValidationSeverity::Critical && severity != ValidationSeverity::Major;
    
    Ok(ValidationResult {
        file_path: fix.file_path.clone(),
        is_valid,
        messages,
        severity,
    })
}

/// Validates syntax of modified code
fn validate_syntax(code: &str) -> ValidationPartialResult {
    let mut result = ValidationPartialResult::new();
    
    // Parse the modified code
    let parsed = SourceFile::parse(code);
    
    // Check for syntax errors
    let syntax = parsed.syntax_node();
    let syntax_errors = syntax.descendants().filter(|node| node.kind() == SyntaxKind::ERROR);
    
    let error_count = syntax_errors.count();
    if error_count > 0 {
        result.add_critical_error(
            format!("Found {} syntax errors in the modified code", error_count),
            None,
        );
    } else {
        result.add_info("Syntax validation passed".to_string(), None);
    }
    
    result
}

/// Validates semantic correctness
fn validate_semantics(file_path: &Path, code: &str) -> ValidationPartialResult {
    let mut result = ValidationPartialResult::new();
    
    // This would ideally run rustc to check for semantic errors
    // Since that's complex, we'll do some basic checks
    
    // Check for unresolved macros
    if code.contains("unresolved_macro!") {
        result.add_error("Code contains unresolved macros".to_string(), None);
    }
    
    // Check for TODO comments
    if code.contains("TODO") || code.contains("FIXME") {
        result.add_warning("Code contains TODO or FIXME comments".to_string(), None);
    }
    
    // Add a success info message if no issues found
    if result.severity == ValidationSeverity::None {
        result.add_info("Semantic validation passed".to_string(), None);
    }
    
    result
}

/// Validates structural integrity between original and modified code
fn validate_structural_integrity(original: &str, modified: &str) -> ValidationPartialResult {
    let mut result = ValidationPartialResult::new();
    
    // Check for preservation of crate features
    let original_features = extract_features(original);
    let modified_features = extract_features(modified);
    
    if original_features != modified_features {
        result.add_error("Crate features were modified".to_string(), None);
    }
    
    // Check for preservation of cfg attributes
    let original_cfgs = extract_cfg_attributes(original);
    let modified_cfgs = extract_cfg_attributes(modified);
    
    if original_cfgs != modified_cfgs {
        result.add_error("Conditional compilation directives were modified".to_string(), None);
    }
    
    // Check for preservation of module structure
    let original_mods = extract_modules(original);
    let modified_mods = extract_modules(modified);
    
    for module in &original_mods {
        if !modified_mods.contains(module) {
            result.add_error(format!("Module '{}' was removed", module), None);
        }
    }
    
    // Add a success info message if no issues found
    if result.severity == ValidationSeverity::None {
        result.add_info("Structural integrity validation passed".to_string(), None);
    }
    
    result
}

/// Validates Tauri compatibility
fn validate_tauri_compatibility(original: &str, modified: &str) -> ValidationPartialResult {
    let mut result = ValidationPartialResult::new();
    
    // Check Tauri command definitions
    let original_commands = extract_tauri_commands(original);
    let modified_commands = extract_tauri_commands(modified);
    
    for cmd in &original_commands {
        if !modified_commands.contains(cmd) {
            result.add_error(format!("Tauri command '{}' was removed", cmd), None);
        }
    }
    
    // Check invoke handler registrations
    let original_handlers = extract_invoke_handlers(original);
    let modified_handlers = extract_invoke_handlers(modified);
    
    for handler in &original_handlers {
        if !modified_handlers.contains(handler) {
            result.add_error(format!("Tauri invoke handler '{}' was removed", handler), None);
        }
    }
    
    // Check all commands are registered
    for cmd in &modified_commands {
        let is_registered = modified_handlers.iter().any(|h| h.contains(cmd));
        if !is_registered {
            result.add_warning(format!("Tauri command '{}' is not registered in any invoke_handler", cmd), None);
        }
    }
    
    // Add a success info message if no issues found
    if result.severity == ValidationSeverity::None {
        result.add_info("Tauri compatibility validation passed".to_string(), None);
    }
    
    result
}

/// Validates security implications
fn validate_security_implications(original: &str, modified: &str) -> ValidationPartialResult {
    let mut result = ValidationPartialResult::new();
    
    // Check for security-critical functions
    let security_functions = [
        "validate_path_safety",
        "verify_signature",
        "encrypt",
        "decrypt",
        "hash",
        "verify",
        "authenticate",
    ];
    
    for func in &security_functions {
        let original_calls = count_function_calls(original, func);
        let modified_calls = count_function_calls(modified, func);
        
        if original_calls != modified_calls {
            result.add_error(format!("Security function '{}' calls were modified", func), None);
        }
    }
    
    // Check for new unsafe blocks
    let original_unsafe = count_unsafe_blocks(original);
    let modified_unsafe = count_unsafe_blocks(modified);
    
    if modified_unsafe > original_unsafe {
        result.add_error(
            format!("Added {} new unsafe blocks", modified_unsafe - original_unsafe),
            None,
        );
    }
    
    // Check for unwrap/expect on security operations
    let sensitive_unwraps = [
        r"verify.*\.unwrap\(\)",
        r"auth.*\.unwrap\(\)",
        r"decrypt.*\.unwrap\(\)",
        r"\.verify.*\.unwrap\(\)",
    ];
    
    for pattern in &sensitive_unwraps {
        let re = regex::Regex::new(pattern).unwrap();
        let original_count = re.find_iter(original).count();
        let modified_count = re.find_iter(modified).count();
        
        if modified_count > original_count {
            result.add_error(
                format!("Added unwrap() on security-sensitive operation matching '{}'", pattern),
                None,
            );
        }
    }
    
    // Add a success info message if no issues found
    if result.severity == ValidationSeverity::None {
        result.add_info("Security validation passed".to_string(), None);
    }
    
    result
}

/// Extracts crate features from code
fn extract_features(code: &str) -> Vec<String> {
    // In a real implementation, this would use actual AST parsing
    // For now, we'll use a simple regex approach
    
    let feature_regex = regex::Regex::new(r"#!\[feature\(([^\)]+)\)\]").unwrap();
    
    feature_regex
        .captures_iter(code)
        .map(|cap| cap[1].to_string())
        .collect()
}

/// Extracts cfg attributes from code
fn extract_cfg_attributes(code: &str) -> Vec<String> {
    // In a real implementation, this would use actual AST parsing
    // For now, we'll use a simple regex approach
    
    let cfg_regex = regex::Regex::new(r"#\[cfg\(([^\)]+)\)\]").unwrap();
    
    cfg_regex
        .captures_iter(code)
        .map(|cap| cap[1].to_string())
        .collect()
}

/// Extracts module declarations from code
fn extract_modules(code: &str) -> Vec<String> {
    // In a real implementation, this would use actual AST parsing
    // For now, we'll use a simple regex approach
    
    let mod_regex = regex::Regex::new(r"mod\s+([a-zA-Z0-9_]+)\s*;").unwrap();
    let mod_block_regex = regex::Regex::new(r"mod\s+([a-zA-Z0-9_]+)\s*\{").unwrap();
    
    let mut modules = Vec::new();
    
    for cap in mod_regex.captures_iter(code) {
        modules.push(cap[1].to_string());
    }
    
    for cap in mod_block_regex.captures_iter(code) {
        modules.push(cap[1].to_string());
    }
    
    modules
}

/// Extracts Tauri commands from code
fn extract_tauri_commands(code: &str) -> Vec<String> {
    // In a real implementation, this would use actual AST parsing
    // For now, we'll use a simple regex approach
    
    let command_regex = regex::Regex::new(r"#\[tauri::command\]\s*(?:pub\s+)?fn\s+([a-zA-Z0-9_]+)").unwrap();
    
    command_regex
        .captures_iter(code)
        .map(|cap| cap[1].to_string())
        .collect()
}

/// Extracts Tauri invoke handlers from code
fn extract_invoke_handlers(code: &str) -> Vec<String> {
    // In a real implementation, this would use actual AST parsing
    // For now, we'll use a simple regex approach
    
    let handler_regex = regex::Regex::new(r"\.invoke_handler\(([^)]+)\)").unwrap();
    
    handler_regex
        .captures_iter(code)
        .map(|cap| cap[1].to_string())
        .collect()
}

/// Counts function calls in code
fn count_function_calls(code: &str, function_name: &str) -> usize {
    // In a real implementation, this would use actual AST parsing
    // For now, we'll use a simple regex approach
    
    let call_regex = regex::Regex::new(&format!(r"{}\s*\(", function_name)).unwrap();
    
    call_regex.captures_iter(code).count()
}

/// Counts unsafe blocks in code
fn count_unsafe_blocks(code: &str) -> usize {
    // In a real implementation, this would use actual AST parsing
    // For now, we'll use a simple regex approach
    
    let unsafe_regex = regex::Regex::new(r"unsafe\s*\{").unwrap();
    
    unsafe_regex.captures_iter(code).count()
}

/// Checks if a file is part of a Tauri project
fn is_tauri_file(file_path: &Path) -> bool {
    // Check if the file is in a src-tauri directory
    let path_str = file_path.to_string_lossy();
    path_str.contains("src-tauri") || path_str.contains("tauri.conf.json")
}