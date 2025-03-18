# Rust AI-Powered Project Analyzer & Code Refactoring Tool

A powerful code analysis and refactoring tool that leverages AI to help you analyze, improve, and generate Rust projects.

## 🚀 Features

- **AI-powered code analysis** - Detect syntax issues, anti-patterns, security vulnerabilities, and performance bottlenecks
- **Automated code refactoring** - Apply suggested fixes with confidence through validation
- **Project generation** - Create new Rust projects from descriptions
- **GitHub integration** - Clone repositories, create pull requests, and analyze projects directly from GitHub
- **Multiple AI models** - Support for Claude, GPT, Mistral, and local models via Ollama
- **Tauri compatibility validation** - Ensure changes don't break Tauri-specific code
- **Python bindings** - Use the tool from Python code

## 📦 Installation

### Prerequisites

- Rust 1.65 or newer
- Python 3.8 or newer (for Python bindings)
- Git

### Building from source

```bash
# Clone the repository
git clone https://github.com/yourusername/rust-ai-tool.git
cd rust-ai-tool

# Build with Cargo
cargo build --release

# Add to PATH (optional)
export PATH="$PATH:$(pwd)/target/release"
```

### Using with Python

```bash
# Install the Python package
pip install rust-ai-tool

# Or use the Python bindings directly
cd rust-ai-tool
python -m bindings.python_api
```

## 🔧 Usage

### Command-line interface

```bash
# Analyze a Rust project
rust-ai-tool analyze path/to/project

# Validate suggested fixes
rust-ai-tool validate path/to/project --fixes fixes.json

# Apply fixes
rust-ai-tool apply path/to/project --fixes fixes.json --backup

# Generate a new project
rust-ai-tool generate --description "A CLI tool for converting CSV to JSON" --output ./projects --name csv2json

# GitHub integration
rust-ai-tool github create-pr --owner username --repo repository --branch fixes --title "Fix issues" --fixes fixes.json
```

### Python API

```python
from rust_ai_tool import RustAiTool

# Initialize the tool
tool = RustAiTool()

# Analyze a Rust project
analysis_results = tool.analyze_project("path/to/rust/project")

# Apply suggested fixes
tool.apply_fixes("path/to/rust/project", analysis_results)

# Generate a new project from description
tool.generate_project("A CLI tool for converting CSV to JSON", "output_dir", "csv2json")
```

## 🧩 Architecture

The tool is designed with a modular architecture:

- **Rust Core**: High-performance analysis and validation
- **Python Bindings**: AI integration and scripting
- **GitHub Automation**: Repository and PR management
- **Project Generation**: Template-based and AI-assisted project creation

## 🔍 How It Works

1. **Analysis**: The tool scans your Rust code using [rust-analyzer](https://rust-analyzer.github.io/) and Clippy, collecting issues and improvement opportunities.

2. **AI Processing**: The collected issues are sent to an AI model (Claude, GPT, or Mistral) for analysis and fix generation.

3. **Validation**: Suggested fixes are validated to ensure they maintain correct syntax, preserve semantics, and don't break Tauri-specific functionality.

4. **Application**: Validated fixes are applied to your code, with backups created for safety.

## 📝 Configuration

Create a `.rust-ai-tool.toml` file in your project directory:

```toml
[ai]
model_type = "Claude" # Claude, Gpt, Mistral, or Local
api_key = "your-api-key"
# api_base_url = "https://custom-endpoint" # Optional

[analysis]
run_clippy = true
use_rust_analyzer = true

[validation]
syntax_only = false
tauri_compatibility = true
security_validation = true

[github]
# GitHub integration settings (optional)
owner = "username"
repo = "repository"
token = "your-github-token"
```

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## 📄 License

This project is licensed under the MIT License - see the LICENSE file for details.