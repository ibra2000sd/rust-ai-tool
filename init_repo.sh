#!/bin/bash
# Initialize the Rust AI-Powered Project Analyzer & Code Refactoring Tool repository

set -e  # Exit on error

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[0;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${BLUE}======================================================${NC}"
echo -e "${BLUE}  Initializing Rust AI Tool Repository  ${NC}"
echo -e "${BLUE}======================================================${NC}"

# Check if git is installed
if ! command -v git &> /dev/null; then
    echo -e "${RED}Error: git is not installed. Please install git first.${NC}"
    exit 1
fi

# Check if cargo is installed
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}Error: cargo is not installed. Please install Rust and Cargo first.${NC}"
    exit 1
fi

# Create GitHub repository
echo -e "${BLUE}Step 1: Creating GitHub repository${NC}"
echo -e "${YELLOW}Please enter your GitHub username:${NC}"
read github_username

# Check if the repository already exists locally
if [ -d "rust-ai-tool" ]; then
    echo -e "${YELLOW}Warning: 'rust-ai-tool' directory already exists.${NC}"
    echo -e "${YELLOW}Do you want to remove it and create a new one? (y/n)${NC}"
    read should_remove
    if [ "$should_remove" = "y" ]; then
        rm -rf rust-ai-tool
    else
        echo -e "${RED}Aborting.${NC}"
        exit 1
    fi
fi

echo -e "${GREEN}Creating local repository...${NC}"
mkdir -p rust-ai-tool
cd rust-ai-tool

# Initialize git
git init

# Copy project structure
echo -e "${BLUE}Step 2: Setting up project structure${NC}"

mkdir -p src/models
mkdir -p bindings
mkdir -p .github/workflows

# Create core files
echo -e "${GREEN}Creating source files...${NC}"

echo -e "${YELLOW}Do you want to create a remote GitHub repository? (y/n)${NC}"
read create_remote

if [ "$create_remote" = "y" ]; then
    echo -e "${YELLOW}Please enter your GitHub personal access token:${NC}"
    read github_token
    
    echo -e "${GREEN}Creating remote repository...${NC}"
    
    # Create repository on GitHub
    response=$(curl -s -X POST \
        -H "Authorization: token $github_token" \
        -H "Accept: application/vnd.github.v3+json" \
        https://api.github.com/user/repos \
        -d '{"name":"rust-ai-tool","description":"Rust AI-Powered Project Analyzer & Code Refactoring Tool","private":false}')
    
    # Check if repository was created successfully
    if echo "$response" | grep -q "ssh_url"; then
        echo -e "${GREEN}Remote repository created successfully.${NC}"
        
        # Add remote
        git remote add origin "https://github.com/$github_username/rust-ai-tool.git"
        
        # Set default branch to main
        git checkout -b main
    else
        echo -e "${RED}Failed to create remote repository.${NC}"
        echo -e "${RED}Response: $response${NC}"
        echo -e "${YELLOW}Continuing with local repository only.${NC}"
    fi
else
    # Set default branch to main
    git checkout -b main
fi

# Initialize cargo
echo -e "${BLUE}Step 3: Initializing Cargo project${NC}"
cargo init --lib

# Commit initial files
echo -e "${BLUE}Step 4: Committing initial files${NC}"
git add .
git commit -m "Initial commit"

# Push to GitHub if remote was set up
if [ "$create_remote" = "y" ] && git remote -v | grep -q origin; then
    echo -e "${BLUE}Step 5: Pushing to GitHub${NC}"
    git push -u origin main
fi

echo -e "${GREEN}======================================================${NC}"
echo -e "${GREEN}  Repository initialized successfully!  ${NC}"
echo -e "${GREEN}======================================================${NC}"
echo -e "${BLUE}Next steps:${NC}"
echo -e "${BLUE}1. Add your code to the repository${NC}"
echo -e "${BLUE}2. Install dependencies: ${YELLOW}cargo build${NC}"
echo -e "${BLUE}3. Run tests: ${YELLOW}cargo test${NC}"
echo -e "${BLUE}4. Start development: ${YELLOW}cargo run${NC}"
echo -e "${GREEN}Happy coding!${NC}"