//! Code modification module
//!
//! This module provides functionality to apply changes to Rust code:
//! - Apply AI-suggested fixes
//! - Apply refactorings
//! - Handle batch modifications
//! - Track changes

use crate::{Result, RustAiToolError};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use log::{debug, info, warn, error};
use serde::{Serialize, Deserialize};

/// Represents a code modification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeModification {
    /// Path to the file to modify
    pub file_path: PathBuf,
    
    /// Original content
    pub original_content: String,
    
    /// Modified content
    pub modified_content: String,
    
    /// Description of the modification
    pub description: String,
    
    /// Confidence level (0-100)
    pub confidence: u8,
}

/// Represents a change in a file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChange {
    /// Path to the file
    pub file_path: PathBuf,
    
    /// Original content (if available)
    pub original_content: Option<String>,
    
    /// New content
    pub new_content: String,
    
    /// Description of the change
    pub description: String,
    
    /// Whether a backup was created
    pub backup_created: bool,
    
    /// Path to the backup file (if created)
    pub backup_path: Option<PathBuf>,
}

/// Apply a list of code modifications
///
/// # Arguments
///
/// * `modifications` - List of modifications to apply
/// * `create_backup` - Whether to create backups of modified files
///
/// # Returns
///
/// List of applied changes
pub fn apply_modifications(
    modifications: &[CodeModification],
    create_backup: bool,
) -> Result<Vec<FileChange>> {
    info!("Applying {} modifications with backup={}", modifications.len(), create_backup);
    let mut changes = Vec::new();
    
    for (i, modification) in modifications.iter().enumerate() {
        debug!("Applying modification #{} to {}", i + 1, modification.file_path.display());
        match apply_modification(modification, create_backup) {
            Ok(change) => {
                info!("Successfully applied modification to {}", modification.file_path.display());
                changes.push(change);
            },
            Err(e) => {
                error!(
                    "Failed to apply modification to {}: {}",
                    modification.file_path.display(),
                    e
                );
                return Err(e);
            }
        }
    }
    
    info!("Successfully applied {} modifications", changes.len());
    Ok(changes)
}

/// Apply a single code modification
///
/// # Arguments
///
/// * `modification` - Modification to apply
/// * `create_backup` - Whether to create a backup of the modified file
///
/// # Returns
///
/// The file change
fn apply_modification(
    modification: &CodeModification,
    create_backup: bool,
) -> Result<FileChange> {
    let file_path = &modification.file_path;
    
    // Check if the file exists
    if !file_path.exists() {
        return Err(RustAiToolError::Modification(format!(
            "File not found: {}",
            file_path.display()
        )));
    }
    
    // Read the current content
    let current_content = fs::read_to_string(file_path)
        .map_err(|e| RustAiToolError::Io(e))?;
    
    // Compare with the original content to make sure it hasn't changed
    if current_content != modification.original_content {
        return Err(RustAiToolError::Modification(format!(
            "File {} has been modified since the original content was read",
            file_path.display()
        )));
    }
    
    // Create a backup if requested
    let backup_path = if create_backup {
        let backup_file = file_path.with_extension("bak");
        fs::write(&backup_file, &current_content)
            .map_err(|e| RustAiToolError::Io(e))?;
        debug!("Created backup at {}", backup_file.display());
        Some(backup_file)
    } else {
        None
    };
    
    // Write the modified content
    fs::write(file_path, &modification.modified_content)
        .map_err(|e| RustAiToolError::Io(e))?;
    
    Ok(FileChange {
        file_path: file_path.to_path_buf(),
        original_content: Some(current_content),
        new_content: modification.modified_content.clone(),
        description: modification.description.clone(),
        backup_created: backup_path.is_some(),
        backup_path,
    })
}

/// Apply validated fixes
///
/// # Arguments
///
/// * `modifications` - List of all modifications
/// * `validation_results` - List of validation results
/// * `create_backup` - Whether to create backups
///
/// # Returns
///
/// List of applied changes
pub fn apply_validated_fixes(
    modifications: &[CodeModification],
    validation_results: &[crate::validation::ValidationResult],
    create_backup: bool,
) -> Result<Vec<FileChange>> {
    // Filter modifications based on validation results
    let valid_modifications: Vec<&CodeModification> = modifications.iter()
        .zip(validation_results.iter())
        .filter(|(_, validation)| validation.is_valid)
        .map(|(modification, _)| modification)
        .collect();
    
    // Log stats
    let valid_count = valid_modifications.len();
    let total_count = modifications.len();
    info!("Applying {}/{} validated fixes", valid_count, total_count);
    
    if valid_count < total_count {
        let invalid_count = total_count - valid_count;
        warn!("Skipping {} invalid modifications", invalid_count);
    }
    
    // Apply only the valid modifications
    let mut changes = Vec::new();
    for modification in valid_modifications {
        match apply_modification(modification, create_backup) {
            Ok(change) => {
                changes.push(change);
            },
            Err(e) => {
                error!(
                    "Failed to apply validated modification to {}: {}",
                    modification.file_path.display(),
                    e
                );
                return Err(e);
            }
        }
    }
    
    Ok(changes)
}

/// Creates a detailed report of changes
///
/// # Arguments
///
/// * `changes` - List of changes to report
///
/// # Returns
///
/// A formatted report of changes
pub fn create_change_report(changes: &[FileChange]) -> String {
    let mut report = String::new();
    
    report.push_str("# Code Modification Report\n\n");
    report.push_str(&format!("Total files modified: {}\n\n", changes.len()));
    
    for (i, change) in changes.iter().enumerate() {
        report.push_str(&format!("## {}. {}\n\n", i + 1, change.file_path.display()));
        report.push_str(&format!("Description: {}\n\n", change.description));
        
        if let Some(original) = &change.original_content {
            report.push_str("### Changes\n\n");
            report.push_str("```diff\n");
            
            // Generate a simple diff
            let diff = generate_diff(original, &change.new_content);
            report.push_str(&diff);
            
            report.push_str("```\n\n");
        }
        
        if change.backup_created {
            report.push_str(&format!(
                "Backup created: {}\n\n",
                change.backup_path.as_ref().unwrap().display()
            ));
        }
        
        report.push_str("---\n\n");
    }
    
    report
}

/// Generate a simple diff between two strings
///
/// # Arguments
///
/// * `original` - Original text
/// * `modified` - Modified text
///
/// # Returns
///
/// Diff in unified format
fn generate_diff(original: &str, modified: &str) -> String {
    // This is a simple implementation
    // A real implementation would use a proper diff algorithm
    let original_lines: Vec<&str> = original.lines().collect();
    let modified_lines: Vec<&str> = modified.lines().collect();
    
    let mut diff = String::new();
    
    // Simple line-by-line comparison
    for i in 0..original_lines.len().max(modified_lines.len()) {
        if i < original_lines.len() && i < modified_lines.len() {
            if original_lines[i] != modified_lines[i] {
                diff.push_str(&format!("- {}\n", original_lines[i]));
                diff.push_str(&format!("+ {}\n", modified_lines[i]));
            } else {
                diff.push_str(&format!("  {}\n", original_lines[i]));
            }
        } else if i < original_lines.len() {
            diff.push_str(&format!("- {}\n", original_lines[i]));
        } else if i < modified_lines.len() {
            diff.push_str(&format!("+ {}\n", modified_lines[i]));
        }
    }
    
    diff
}

/// Restore files from backups
///
/// # Arguments
///
/// * `changes` - List of changes to restore
///
/// # Returns
///
/// Number of files restored
pub fn restore_backups(changes: &[FileChange]) -> Result<usize> {
    let mut restored = 0;
    
    for change in changes {
        if let Some(backup_path) = &change.backup_path {
            if backup_path.exists() {
                // Read the backup content
                let backup_content = fs::read_to_string(backup_path)
                    .map_err(|e| RustAiToolError::Io(e))?;
                
                // Write it back to the original file
                fs::write(&change.file_path, backup_content)
                    .map_err(|e| RustAiToolError::Io(e))?;
                
                // Remove the backup file
                fs::remove_file(backup_path).map_err(|e| RustAiToolError::Io(e))?;
                
                restored += 1;
                info!("Restored {} from backup", change.file_path.display());
            } else {
                warn!("Backup file not found: {}", backup_path.display());
            }
        }
    }
    
    Ok(restored)
}

/// Apply changes to multiple files
///
/// # Arguments
///
/// * `changes` - Map of file paths to content changes
/// * `create_backup` - Whether to create backups
///
/// # Returns
///
/// List of file changes
pub fn apply_file_changes(
    changes: &HashMap<PathBuf, String>,
    create_backup: bool,
) -> Result<Vec<FileChange>> {
    let mut file_changes = Vec::new();
    
    for (file_path, new_content) in changes {
        // Check if the file exists
        if !file_path.exists() {
            return Err(RustAiToolError::Modification(format!(
                "File not found: {}",
                file_path.display()
            )));
        }
        
        // Read the current content
        let current_content = fs::read_to_string(file_path)
            .map_err(|e| RustAiToolError::Io(e))?;
        
        // Skip if the content is already the same
        if current_content == *new_content {
            info!("Skipping {} - content unchanged", file_path.display());
            continue;
        }
        
        // Create a backup if requested
        let backup_path = if create_backup {
            let backup_file = file_path.with_extension("bak");
            fs::write(&backup_file, &current_content)
                .map_err(|e| RustAiToolError::Io(e))?;
            info!("Created backup at {}", backup_file.display());
            Some(backup_file)
        } else {
            None
        };
        
        // Write the new content
        fs::write(file_path, new_content)
            .map_err(|e| RustAiToolError::Io(e))?;
        
        info!("Updated {}", file_path.display());
        
        file_changes.push(FileChange {
            file_path: file_path.clone(),
            original_content: Some(current_content),
            new_content: new_content.clone(),
            description: "Modified file content".to_string(),
            backup_created: backup_path.is_some(),
            backup_path,
        });
    }
    
    Ok(file_changes)
}

/// Safely update a section of code in a file
///
/// # Arguments
///
/// * `file_path` - Path to the file
/// * `search_text` - Text to search for
/// * `replacement` - Replacement text
/// * `create_backup` - Whether to create a backup
///
/// # Returns
///
/// File change if successful
pub fn update_code_section(
    file_path: &Path,
    search_text: &str,
    replacement: &str,
    create_backup: bool,
) -> Result<FileChange> {
    // Read the current content
    let current_content = fs::read_to_string(file_path)
        .map_err(|e| RustAiToolError::Io(e))?;
    
    // Check if the search text exists
    if !current_content.contains(search_text) {
        return Err(RustAiToolError::Modification(format!(
            "Search text not found in {}",
            file_path.display()
        )));
    }
    
    // Replace the text
    let new_content = current_content.replace(search_text, replacement);
    
    // Create a backup if requested
    let backup_path = if create_backup {
        let backup_file = file_path.with_extension("bak");
        fs::write(&backup_file, &current_content)
            .map_err(|e| RustAiToolError::Io(e))?;
        debug!("Created backup at {}", backup_file.display());
        Some(backup_file)
    } else {
        None
    };
    
    // Write the new content
    fs::write(file_path, &new_content)
        .map_err(|e| RustAiToolError::Io(e))?;
    
    info!("Updated section in {}", file_path.display());
    
    Ok(FileChange {
        file_path: file_path.to_path_buf(),
        original_content: Some(current_content),
        new_content,
        description: format!("Updated code section in {}", file_path.display()),
        backup_created: backup_path.is_some(),
        backup_path,
    })
}

/// Create a code modification from original and modified content
///
/// # Arguments
///
/// * `file_path` - Path to the file
/// * `original_content` - Original content
/// * `modified_content` - Modified content
/// * `description` - Description of the modification
/// * `confidence` - Confidence level (0-100)
///
/// # Returns
///
/// Code modification
pub fn create_modification(
    file_path: PathBuf,
    original_content: String,
    modified_content: String,
    description: String,
    confidence: u8,
) -> CodeModification {
    CodeModification {
        file_path,
        original_content,
        modified_content,
        description,
        confidence,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[test]
    fn test_apply_modification() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.rs");
        
        let original_content = "fn main() {\n    println!(\"Hello\");\n}";
        fs::write(&file_path, original_content).unwrap();
        
        let modified_content = "fn main() {\n    println!(\"Hello, world!\");\n}";
        
        let modification = CodeModification {
            file_path: file_path.clone(),
            original_content: original_content.to_string(),
            modified_content: modified_content.to_string(),
            description: "Update greeting".to_string(),
            confidence: 90,
        };
        
        let change = apply_modification(&modification, true).unwrap();
        
        assert_eq!(change.file_path, file_path);
        assert_eq!(change.original_content, Some(original_content.to_string()));
        assert_eq!(change.new_content, modified_content);
        assert!(change.backup_created);
        assert!(change.backup_path.is_some());
        
        // Check that the file was updated
        let updated_content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(updated_content, modified_content);
        
        // Check that the backup was created
        let backup_path = file_path.with_extension("bak");
        let backup_content = fs::read_to_string(&backup_path).unwrap();
        assert_eq!(backup_content, original_content);
    }
}