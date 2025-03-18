//! Validation module for suggested code fixes
//!
//! This module provides functionality to validate suggested code fixes:
//! - Syntax validation
//! - Semantic validation
//! - Security implications
//! - Tauri compatibility
//! - Structural integrity

use crate::{RustAiToolError, ValidationOptions, Result};
use ra_ap_syntax::{SourceFile, SyntaxNode};
use std::path::{Path, PathBuf};

/// Result of validating a fix
#[derive(Debug)]
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
#[derive(Debug)]
pub struct ValidationMessage {
    /// Type of validation message
    pub message_type: ValidationMessageType,
    
    /// The message text
    pub text: String,
    
    /// Location in the code (if relevant)
    pub location: Option<CodeLocation>,
}

/// Location in code
#[derive(Debug)]
pub struct CodeLocation {
    /// Line number
    pub line: usize,
    
    /// Column number
    pub column: usize,
}

/// Types of validation messages
#[derive(Debug, PartialEq)]
pub enum ValidationMessageType {
    /// Error that prevents applying the fix
    Error,
    
    /// Warning that might indicate a problem
    Warning,
    
    /// Informational message
    Info,
}

/// Severity of validation issues
#[derive(Debug, PartialEq)]
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

/// Represents a code fix to validate
#[derive(Debug)]
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
    let mut results = Vec::new();
    
    for fix in fixes {
        match validate_fix(fix, options) {
            Ok(result) => results.push(result),
            Err(e) => {
                // Log error but continue with other fixes
                log::error!("Failed to validate fix for {}: {}", fix.file_path.display(), e);
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
fn validate_fix(fix: &FixToValidate, options: &ValidationOptions) -> Result<ValidationResult> {
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
    let mut messages = Vec::new();
    let mut severity = ValidationSeverity::None;
    
    // Parse the modified code
    let parsed = SourceFile::parse(code);
    
    // Check for syntax errors
    let syntax = parsed.syntax_node();
    let syntax_errors = syntax.descendants().filter(|node| node.kind() == ra_ap_syntax::SyntaxKind::ERROR);
    
    for error in syntax_errors {
        severity = ValidationSeverity::Critical;
        
        messages.push(ValidationMessage {
            message_type: ValidationMessageType::Error,
            text: format!("Syntax error at {:?}", error.text_range()),
            location: None, // In a real implementation, calculate line/column
        });
    }
    
    ValidationPartialResult {
        messages,
        severity,
    }
}

/// Validates semantic correctness
fn validate_semantics(file_path: &Path, code: &str) -> ValidationPartialResult {
    // In a real implementation, this would run rustc to check for semantic errors
    // For now, we'll just return an empty result
    
    ValidationPartialResult {
        messages: Vec::new(),
        severity: ValidationSeverity::None,
    }
}

/// Validates structural integrity between original and modified code
fn validate_structural_integrity(original: &str, modified: &str) -> ValidationPartialResult {
    let mut messages = Vec::new();
    let mut severity = ValidationSeverity::None;
    
    // Check for preservation of crate features
    let original_features = extract_features(original);
    let modified_features = extract_features(modified);
    
    if original_features != modified_features {
        severity = ValidationSeverity::Major;
        
        messages.push(ValidationMessage {
            message_type: ValidationMessageType::Error,
            text: "Crate features were modified".to_string(),
            location: None,
        });
    }
    
    // Check for preservation of cfg attributes
    let original_cfgs = extract_cfg_attributes(original);
    let modified_cfgs = extract_cfg_attributes(modified);
    
    if original_cfgs != modified_cfgs {
        severity = ValidationSeverity::Major;
        
        messages.push(ValidationMessage {
            message_type: ValidationMessageType::Error,
            text: "Conditional compilation directives were modified".to_string(),
            location: None,
        });
    }
    
    ValidationPartialResult {
        messages,
        severity,
    }
}

/// Validates Tauri compatibility
fn validate_tauri_compatibility(original: &str, modified: &str) -> ValidationPartialResult {
    let mut messages = Vec::new();
    let mut severity = ValidationSeverity::None;
    
    // Check Tauri command definitions
    let original_commands = extract_tauri_commands(original);
    let modified_commands = extract_tauri_commands(modified);
    
    if original_commands != modified_commands {
        severity = ValidationSeverity::Major;
        
        messages.push(ValidationMessage {
            message_type: ValidationMessageType::Error,
            text: "Tauri command definitions were modified".to_string(),
            location: None,
        });
    }
    
    // Check invoke handler registrations
    let original_handlers = extract_invoke_handlers(original);
    let modified_handlers = extract_invoke_handlers(modified);
    
    if original_handlers != modified_handlers {
        severity = ValidationSeverity::Major;
        
        messages.push(ValidationMessage {
            message_type: ValidationMessageType::Error,
            text: "Tauri invoke handler registrations were modified".to_string(),
            location: None,
        });
    }
    
    ValidationPartialResult {
        messages,
        severity,
    }
}

/// Validates security implications
fn validate_security_implications(original: &str, modified: &str) -> ValidationPartialResult {
    let mut messages = Vec::new();
    let mut severity = ValidationSeverity::None;
    
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
            severity = ValidationSeverity::Major;
            
            messages.push(ValidationMessage {
                message_type: ValidationMessageType::Error,
                text: format!("Security function '{}' calls were modified", func),
                location: None,
            });
        }
    }
    
    ValidationPartialResult {
        messages,
        severity,
    }
}

/// Helper struct for partial validation results
#[derive(Debug)]
struct ValidationPartialResult {
    messages: Vec<ValidationMessage>,
    severity: ValidationSeverity,
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

/// Checks if a file is part of a Tauri project
fn is_tauri_file(file_path: &Path) -> bool {
    // Check if the file is in a src-tauri directory
    let path_str = file_path.to_string_lossy();
    path_str.contains("src-tauri") || path_str.contains("tauri.conf.json")
}