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

/// Represents a code modification
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
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
    let mut changes = Vec::new();
    
    for modification in modifications {
        match apply_modification(modification, create_backup) {
            Ok(change) => changes.push(change),
            Err(e) => {
                log::error!(
                    "Failed to apply modification to {}: {}",
                    modification.file_path.display(),
                    e
                );
                return Err(e);
            }
        }
    }
    
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
            
            // Create a simple diff
            let original_lines: Vec<&str> = original.lines().collect();
            let new_lines: Vec<&str> = change.new_content.lines().collect();
            
            // Very simple diff algorithm
            for i in 0..original_lines.len().max(new_lines.len()) {
                if i < original_lines.len() && i < new_lines.len() {
                    if original_lines[i] != new_lines[i] {
                        report.push_str(&format!("- {}\n", original_lines[i]));
                        report.push_str(&format!("+ {}\n", new_lines[i]));
                    }
                } else if i < original_lines.len() {
                    report.push_str(&format!("- {}\n", original_lines[i]));
                } else if i < new_lines.len() {
                    report.push_str(&format!("+ {}\n", new_lines[i]));
                }
            }
            
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
            log::info!("Skipping {} - content unchanged", file_path.display());
            continue;
        }
        
        // Create a backup if requested
        let backup_path = if create_backup {
            let backup_file = file_path.with_extension("bak");
            fs::write(&backup_file, &current_content)
                .map_err(|e| RustAiToolError::Io(e))?;
            Some(backup_file)
        } else {
            None
        };
        
        // Write the new content
        fs::write(file_path, new_content)
            .map_err(|e| RustAiToolError::Io(e))?;
        
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
        Some(backup_file)
    } else {
        None
    };
    
    // Write the new content
    fs::write(file_path, &new_content)
        .map_err(|e| RustAiToolError::Io(e))?;
    
    Ok(FileChange {
        file_path: file_path.to_path_buf(),
        original_content: Some(current_content),
        new_content,
        description: format!("Updated code section in {}", file_path.display()),
        backup_created: backup_path.is_some(),
        backup_path,
    })
}