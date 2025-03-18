#!/usr/bin/env python3
"""
Python API for Rust AI-Powered Project Analyzer & Code Refactoring Tool

This module provides Python bindings for the Rust AI Tool, allowing for:
- AI-powered code analysis and refactoring
- Project generation and modification
- Integration with AI models

Example usage:
```python
from python_api import RustAiTool

# Initialize the tool
tool = RustAiTool()

# Analyze a Rust project
analysis_results = tool.analyze_project("path/to/rust/project")

# Apply suggested fixes
tool.apply_fixes("path/to/rust/project", analysis_results)

# Generate a new project from description
tool.generate_project("A CLI tool for converting CSV to JSON", "output_dir", "csv2json")
```
"""

import os
import sys
import json
import subprocess
import tempfile
from typing import Dict, List, Optional, Union, Any

class RustAiTool:
    """Python interface to the Rust AI Tool"""
    
    def __init__(self, rust_binary_path: Optional[str] = None, config_path: Optional[str] = None):
        """
        Initialize the Rust AI Tool

        Args:
            rust_binary_path: Path to the Rust binary (default: auto-detect)
            config_path: Path to the configuration file (default: .rust-ai-tool.toml)
        """
        self.rust_binary_path = rust_binary_path or self._find_rust_binary()
        self.config_path = config_path or ".rust-ai-tool.toml"
        
        # Verify the binary exists
        if not os.path.exists(self.rust_binary_path):
            raise FileNotFoundError(f"Rust AI Tool binary not found at {self.rust_binary_path}")
    
    def _find_rust_binary(self) -> str:
        """Find the Rust binary in PATH or relative to this script"""
        # Try to find in the same directory as this script
        script_dir = os.path.dirname(os.path.abspath(__file__))
        parent_dir = os.path.dirname(script_dir)
        
        potential_paths = [
            os.path.join(parent_dir, "target", "release", "rust-ai-tool"),
            os.path.join(parent_dir, "target", "release", "rust-ai-tool.exe"),
            os.path.join(parent_dir, "target", "debug", "rust-ai-tool"),
            os.path.join(parent_dir, "target", "debug", "rust-ai-tool.exe"),
        ]
        
        for path in potential_paths:
            if os.path.exists(path):
                return path
        
        # Try to find in PATH
        try:
            result = subprocess.run(
                ["which", "rust-ai-tool"] if sys.platform != "win32" else ["where", "rust-ai-tool"],
                capture_output=True,
                text=True,
                check=True
            )
            return result.stdout.strip()
        except (subprocess.CalledProcessError, FileNotFoundError):
            # Default to assuming it's in the PATH without full path
            return "rust-ai-tool"
    
    def _run_command(self, args: List[str], input_data: Optional[str] = None) -> str:
        """
        Run a command with the Rust binary

        Args:
            args: Command arguments
            input_data: Optional input data for stdin

        Returns:
            Command output as string
        """
        cmd = [self.rust_binary_path, "--config", self.config_path] + args
        
        print(f"Running command: {' '.join(cmd)}")
        
        try:
            result = subprocess.run(
                cmd,
                input=input_data.encode("utf-8") if input_data else None,
                capture_output=True,
                text=True,
                check=True
            )
            return result.stdout
        except subprocess.CalledProcessError as e:
            # If the command failed, raise an exception with the error message
            error_message = e.stderr or e.stdout
            raise RuntimeError(f"Command failed with exit code {e.returncode}: {error_message}")
    
    def analyze_project(self, project_path: str, output_format: str = "json") -> Dict[str, Any]:
        """
        Analyze a Rust project for issues and improvement opportunities

        Args:
            project_path: Path to the Rust project
            output_format: Output format (json, markdown, console)

        Returns:
            Analysis results as dictionary
        """
        args = ["analyze", project_path, "--output", output_format]
        output = self._run_command(args)
        
        if output_format == "json":
            return json.loads(output)
        else:
            return {"output": output}
    
    def validate_fixes(self, project_path: str, fixes: Union[str, Dict[str, Any]]) -> Dict[str, Any]:
        """
        Validate suggested fixes for a Rust project

        Args:
            project_path: Path to the Rust project
            fixes: Fixes as JSON string or dictionary

        Returns:
            Validation results as dictionary
        """
        with tempfile.NamedTemporaryFile(mode='w', suffix='.json', delete=False) as f:
            if isinstance(fixes, dict):
                json.dump(fixes, f)
            else:
                f.write(fixes)
            fixes_path = f.name
        
        try:
            args = ["validate", project_path, "--fixes", fixes_path]
            output = self._run_command(args)
            return json.loads(output)
        finally:
            os.unlink(fixes_path)
    
    def apply_fixes(
        self,
        project_path: str,
        fixes: Union[str, Dict[str, Any]],
        create_backup: bool = True
    ) -> Dict[str, Any]:
        """
        Apply suggested fixes to a Rust project

        Args:
            project_path: Path to the Rust project
            fixes: Fixes as JSON string or dictionary
            create_backup: Whether to create backups of modified files

        Returns:
            Application results as dictionary
        """
        with tempfile.NamedTemporaryFile(mode='w', suffix='.json', delete=False) as f:
            if isinstance(fixes, dict):
                json.dump(fixes, f)
            else:
                f.write(fixes)
            fixes_path = f.name
        
        try:
            args = ["apply", project_path, "--fixes", fixes_path]
            if create_backup:
                args.append("--backup")
            
            output = self._run_command(args)
            return json.loads(output)
        finally:
            os.unlink(fixes_path)
    
    def generate_project(
        self,
        description: str,
        output_dir: str,
        name: str
    ) -> Dict[str, Any]:
        """
        Generate a new Rust project from description

        Args:
            description: Project description
            output_dir: Output directory
            name: Project name

        Returns:
            Generation results as dictionary
        """
        args = [
            "generate",
            "--description", description,
            "--output", output_dir,
            "--name", name
        ]
        
        output = self._run_command(args)
        return json.loads(output)
    
    def create_github_pr(
        self,
        owner: str,
        repo: str,
        branch: str,
        title: str,
        fixes: Union[str, Dict[str, Any]]
    ) -> Dict[str, Any]:
        """
        Create a GitHub pull request with suggested fixes

        Args:
            owner: Repository owner
            repo: Repository name
            branch: Branch name
            title: Pull request title
            fixes: Fixes as JSON string or dictionary

        Returns:
            Pull request information as dictionary
        """
        with tempfile.NamedTemporaryFile(mode='w', suffix='.json', delete=False) as f:
            if isinstance(fixes, dict):
                json.dump(fixes, f)
            else:
                f.write(fixes)
            fixes_path = f.name
        
        try:
            args = [
                "github", "create-pr",
                "--owner", owner,
                "--repo", repo,
                "--branch", branch,
                "--title", title,
                "--fixes", fixes_path
            ]
            
            output = self._run_command(args)
            return json.loads(output)
        finally:
            os.unlink(fixes_path)
    
    def analyze_github_repo(
        self,
        owner: str,
        repo: str,
        branch: str = "main"
    ) -> Dict[str, Any]:
        """
        Analyze a GitHub repository

        Args:
            owner: Repository owner
            repo: Repository name
            branch: Branch name

        Returns:
            Analysis results as dictionary
        """
        args = [
            "github", "analyze",
            "--owner", owner,
            "--repo", repo,
            "--branch", branch
        ]
        
        output = self._run_command(args)
        return json.loads(output)

class RustAiToolClient:
    """Client for the Rust AI Tool API"""
    
    def __init__(self, api_url: str = "http://localhost:8080", api_key: Optional[str] = None):
        """
        Initialize the Rust AI Tool API client

        Args:
            api_url: URL of the API server
            api_key: API key for authentication
        """
        import requests
        self.api_url = api_url.rstrip("/")
        self.api_key = api_key
        self.session = requests.Session()
        
        if api_key:
            self.session.headers.update({"Authorization": f"Bearer {api_key}"})
    
    def _make_request(self, method: str, endpoint: str, data: Optional[Dict[str, Any]] = None) -> Dict[str, Any]:
        """
        Make a request to the API

        Args:
            method: HTTP method
            endpoint: API endpoint
            data: Request data

        Returns:
            Response as dictionary
        """
        url = f"{self.api_url}/{endpoint.lstrip('/')}"
        
        if method == "GET":
            response = self.session.get(url, params=data)
        elif method == "POST":
            response = self.session.post(url, json=data)
        elif method == "PUT":
            response = self.session.put(url, json=data)
        elif method == "DELETE":
            response = self.session.delete(url, json=data)
        else:
            raise ValueError(f"Unsupported HTTP method: {method}")
        
        response.raise_for_status()
        return response.json()
    
    def analyze_project(self, project_path: str) -> Dict[str, Any]:
        """
        Analyze a Rust project

        Args:
            project_path: Path to the Rust project

        Returns:
            Analysis results as dictionary
        """
        return self._make_request("POST", "/api/analyze", {"project_path": project_path})
    
    def validate_fixes(self, project_path: str, fixes: Dict[str, Any]) -> Dict[str, Any]:
        """
        Validate suggested fixes

        Args:
            project_path: Path to the Rust project
            fixes: Fixes as dictionary

        Returns:
            Validation results as dictionary
        """
        return self._make_request("POST", "/api/validate", {
            "project_path": project_path,
            "fixes": fixes
        })
    
    def apply_fixes(self, project_path: str, fixes: Dict[str, Any], create_backup: bool = True) -> Dict[str, Any]:
        """
        Apply suggested fixes

        Args:
            project_path: Path to the Rust project
            fixes: Fixes as dictionary
            create_backup: Whether to create backups

        Returns:
            Application results as dictionary
        """
        return self._make_request("POST", "/api/apply", {
            "project_path": project_path,
            "fixes": fixes,
            "create_backup": create_backup
        })
    
    def generate_project(self, description: str, output_dir: str, name: str) -> Dict[str, Any]:
        """
        Generate a new Rust project

        Args:
            description: Project description
            output_dir: Output directory
            name: Project name

        Returns:
            Generation results as dictionary
        """
        return self._make_request("POST", "/api/generate", {
            "description": description,
            "output_dir": output_dir,
            "name": name
        })

def integrate_with_claude(
    api_key: str,
    task: str,
    code: str,
    max_tokens: int = 4000,
    temperature: float = 0.7
) -> str:
    """
    Integrate with Claude AI for code analysis and generation

    Args:
        api_key: Claude API key
        task: Task description
        code: Rust code
        max_tokens: Maximum number of tokens to generate
        temperature: Temperature (randomness)

    Returns:
        Claude's response
    """
    import anthropic
    
    client = anthropic.Anthropic(api_key=api_key)
    
    prompt = f"""
    Task: {task}
    
    Rust code:
    ```rust
    {code}
    ```
    
    Please provide detailed analysis and suggestions:
    """
    
    response = client.completions.create(
        prompt=prompt,
        model="claude-3-opus-20240229",
        max_tokens_to_sample=max_tokens,
        temperature=temperature
    )
    
    return response.completion

def integrate_with_gpt4(
    api_key: str,
    task: str,
    code: str,
    max_tokens: int = 4000,
    temperature: float = 0.7
) -> str:
    """
    Integrate with GPT-4 for code analysis and generation

    Args:
        api_key: OpenAI API key
        task: Task description
        code: Rust code
        max_tokens: Maximum number of tokens to generate
        temperature: Temperature (randomness)

    Returns:
        GPT-4's response
    """
    import openai
    
    client = openai.OpenAI(api_key=api_key)
    
    response = client.chat.completions.create(
        model="gpt-4",
        messages=[
            {"role": "system", "content": "You are a Rust programming expert specializing in code analysis, refactoring, and improvement."},
            {"role": "user", "content": f"""
            Task: {task}
            
            Rust code:
            ```rust
            {code}
            ```
            
            Please provide detailed analysis and suggestions:
            """}
        ],
        max_tokens=max_tokens,
        temperature=temperature
    )
    
    return response.choices[0].message.content

if __name__ == "__main__":
    # Example usage
    print("Rust AI Tool Python API")
    print("This module is intended to be imported, not run directly.")
    print("Example usage:")
    print("""
    from python_api import RustAiTool
    
    # Initialize the tool
    tool = RustAiTool()
    
    # Analyze a Rust project
    analysis_results = tool.analyze_project("path/to/rust/project")
    
    # Apply suggested fixes
    tool.apply_fixes("path/to/rust/project", analysis_results)
    
    # Generate a new project from description
    tool.generate_project("A CLI tool for converting CSV to JSON", "output_dir", "csv2json")
    """)