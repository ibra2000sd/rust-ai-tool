//! AI model integration module
//!
//! This module provides functionality for interacting with AI models:
//! - Integration with Claude AI
//! - Integration with OpenAI GPT models
//! - Integration with Mistral AI
//! - Integration with local models via Ollama

use crate::{AiModelConfig, AiModelType, Result, RustAiToolError};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// AI completion request
#[derive(Debug, Serialize)]
pub struct CompletionRequest {
    /// The prompt for the AI model
    pub prompt: String,
    
    /// Maximum number of tokens to generate
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    
    /// Temperature (randomness)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    
    /// System message/instructions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
}

/// AI completion response
#[derive(Debug, Deserialize)]
pub struct CompletionResponse {
    /// The generated text
    pub content: String,
    
    /// Finish reason
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
    
    /// Usage information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<UsageInfo>,
}

/// Token usage information
#[derive(Debug, Deserialize)]
pub struct UsageInfo {
    /// Number of prompt tokens
    pub prompt_tokens: u32,
    
    /// Number of completion tokens
    pub completion_tokens: u32,
    
    /// Total number of tokens
    pub total_tokens: u32,
}

/// AI model client for generating code and analyzing projects
pub struct AiModelClient {
    /// Configuration for the AI model
    config: AiModelConfig,
    
    /// HTTP client for API requests
    client: reqwest::Client,
}

impl AiModelClient {
    /// Create a new AI model client
    ///
    /// # Arguments
    ///
    /// * `config` - AI model configuration
    ///
    /// # Returns
    ///
    /// A new AI model client
    pub fn new(config: AiModelConfig) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .map_err(|e| RustAiToolError::AiModel(e.to_string()))?;
        
        Ok(Self { config, client })
    }
    
    /// Generate code using the AI model
    ///
    /// # Arguments
    ///
    /// * `prompt` - Prompt for the AI model
    /// * `max_tokens` - Maximum number of tokens to generate
    /// * `temperature` - Temperature (randomness)
    ///
    /// # Returns
    ///
    /// The generated code
    pub async fn generate_code(
        &self,
        prompt: &str,
        max_tokens: Option<u32>,
        temperature: Option<f32>,
    ) -> Result<String> {
        let system = Some(
            "You are a helpful programming assistant that specializes in Rust code. \
            Provide concise, idiomatic Rust code that follows best practices. \
            Include helpful comments to explain your reasoning. \
            When asked to generate or modify code, respond with only the requested code without explanations unless specifically asked."
                .to_string(),
        );
        
        let request = CompletionRequest {
            prompt: prompt.to_string(),
            max_tokens,
            temperature,
            system,
        };
        
        let response = self.send_completion_request(request).await?;
        
        Ok(response.content)
    }
    
    /// Analyze Rust code using the AI model
    ///
    /// # Arguments
    ///
    /// * `code` - Code to analyze
    /// * `instructions` - Instructions for the analysis
    ///
    /// # Returns
    ///
    /// The analysis results
    pub async fn analyze_code(&self, code: &str, instructions: &str) -> Result<String> {
        let system = Some(
            "You are a helpful programming assistant that specializes in analyzing Rust code. \
            Focus on identifying issues related to correctness, performance, security, and idiomatic Rust. \
            Be thorough but concise in your analysis."
                .to_string(),
        );
        
        let prompt = format!(
            "Please analyze the following Rust code:\n\n```rust\n{}\n```\n\n{}",
            code, instructions
        );
        
        let request = CompletionRequest {
            prompt,
            max_tokens: Some(4000),
            temperature: Some(0.2),
            system,
        };
        
        let response = self.send_completion_request(request).await?;
        
        Ok(response.content)
    }
    
    /// Generate fixes for Rust code issues
    ///
    /// # Arguments
    ///
    /// * `code` - Code with issues
    /// * `issues` - Description of the issues
    ///
    /// # Returns
    ///
    /// The fixed code
    pub async fn generate_fixes(&self, code: &str, issues: &str) -> Result<String> {
        let system = Some(
            "You are a helpful programming assistant that specializes in fixing Rust code issues. \
            Provide only the fixed code without explanations unless specifically asked. \
            Ensure your fixes are idiomatic and follow Rust best practices."
                .to_string(),
        );
        
        let prompt = format!(
            "Fix the following issues in this Rust code:\n\nIssues:\n{}\n\nCode:\n```rust\n{}\n```\n\nProvide the fixed code:",
            issues, code
        );
        
        let request = CompletionRequest {
            prompt,
            max_tokens: Some(4000),
            temperature: Some(0.2),
            system,
        };
        
        let response = self.send_completion_request(request).await?;
        
        // Extract code blocks if present
        let code_block_regex = regex::Regex::new(r"```(?:rust)?\s*\n([\s\S]+?)\n```").unwrap();
        if let Some(captures) = code_block_regex.captures(&response.content) {
            if let Some(code_match) = captures.get(1) {
                return Ok(code_match.as_str().to_string());
            }
        }
        
        // Otherwise return the raw response
        Ok(response.content)
    }
    
    /// Generate a Rust project description based on requirements
    ///
    /// # Arguments
    ///
    /// * `requirements` - Project requirements
    ///
    /// # Returns
    ///
    /// The project description
    pub async fn generate_project_description(&self, requirements: &str) -> Result<String> {
        let system = Some(
            "You are a helpful programming assistant that specializes in designing Rust projects. \
            Based on user requirements, create detailed project descriptions including structure, \
            dependencies, and approaches to implementation."
                .to_string(),
        );
        
        let prompt = format!(
            "Generate a detailed Rust project description based on these requirements:\n\n{}\n\n\
            Include suggested crate dependencies, file structure, and implementation approach.",
            requirements
        );
        
        let request = CompletionRequest {
            prompt,
            max_tokens: Some(2000),
            temperature: Some(0.7),
            system,
        };
        
        let response = self.send_completion_request(request).await?;
        
        Ok(response.content)
    }
    
    /// Send a completion request to the AI model
    ///
    /// # Arguments
    ///
    /// * `request` - Completion request
    ///
    /// # Returns
    ///
    /// The completion response
    async fn send_completion_request(
        &self,
        request: CompletionRequest,
    ) -> Result<CompletionResponse> {
        match &self.config.model_type {
            AiModelType::Claude => self.send_claude_request(request).await,
            AiModelType::Gpt => self.send_gpt_request(request).await,
            AiModelType::Mistral => self.send_mistral_request(request).await,
            AiModelType::Local(model_name) => self.send_local_request(request, model_name).await,
        }
    }
    
    /// Send a completion request to Claude AI
    ///
    /// # Arguments
    ///
    /// * `request` - Completion request
    ///
    /// # Returns
    ///
    /// The completion response
    async fn send_claude_request(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        #[derive(Serialize)]
        struct ClaudeRequest {
            model: String,
            prompt: String,
            max_tokens_to_sample: u32,
            temperature: f32,
            system: Option<String>,
        }
        
        #[derive(Deserialize)]
        struct ClaudeResponse {
            completion: String,
        }
        
        let claude_request = ClaudeRequest {
            model: "claude-3-opus-20240229".to_string(), // Use appropriate model version
            prompt: request.prompt,
            max_tokens_to_sample: request.max_tokens.unwrap_or(4000),
            temperature: request.temperature.unwrap_or(0.5),
            system: request.system,
        };
        
        let api_base = self.config.api_base_url.clone().unwrap_or_else(|| {
            "https://api.anthropic.com/v1/complete".to_string()
        });
        
        let response = self
            .client
            .post(&api_base)
            .header("Content-Type", "application/json")
            .header("x-api-key", &self.config.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&claude_request)
            .send()
            .await
            .map_err(|e| RustAiToolError::AiModel(format!("Claude API request failed: {}", e)))?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(RustAiToolError::AiModel(format!(
                "Claude API returned error: {}",
                error_text
            )));
        }
        
        let claude_response = response
            .json::<ClaudeResponse>()
            .await
            .map_err(|e| RustAiToolError::AiModel(format!("Failed to parse Claude response: {}", e)))?;
        
        Ok(CompletionResponse {
            content: claude_response.completion,
            finish_reason: None,
            usage: None,
        })
    }
    
    /// Send a completion request to OpenAI GPT
    ///
    /// # Arguments
    ///
    /// * `request` - Completion request
    ///
    /// # Returns
    ///
    /// The completion response
    async fn send_gpt_request(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        #[derive(Serialize)]
        struct GptMessage {
            role: String,
            content: String,
        }
        
        #[derive(Serialize)]
        struct GptRequest {
            model: String,
            messages: Vec<GptMessage>,
            max_tokens: Option<u32>,
            temperature: Option<f32>,
        }
        
        #[derive(Deserialize)]
        struct GptResponseChoice {
            message: GptMessage,
            finish_reason: Option<String>,
        }
        
        #[derive(Deserialize)]
        struct GptResponseUsage {
            prompt_tokens: u32,
            completion_tokens: u32,
            total_tokens: u32,
        }
        
        #[derive(Deserialize)]
        struct GptResponse {
            choices: Vec<GptResponseChoice>,
            usage: Option<GptResponseUsage>,
        }
        
        let mut messages = Vec::new();
        
        // Add system message if present
        if let Some(system) = request.system {
            messages.push(GptMessage {
                role: "system".to_string(),
                content: system,
            });
        }
        
        // Add user message
        messages.push(GptMessage {
            role: "user".to_string(),
            content: request.prompt,
        });
        
        let gpt_request = GptRequest {
            model: "gpt-4".to_string(), // Use appropriate model version
            messages,
            max_tokens: request.max_tokens,
            temperature: request.temperature,
        };
        
        let api_base = self.config.api_base_url.clone().unwrap_or_else(|| {
            "https://api.openai.com/v1/chat/completions".to_string()
        });
        
        let response = self
            .client
            .post(&api_base)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", &self.config.api_key))
            .json(&gpt_request)
            .send()
            .await
            .map_err(|e| RustAiToolError::AiModel(format!("GPT API request failed: {}", e)))?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(RustAiToolError::AiModel(format!(
                "GPT API returned error: {}",
                error_text
            )));
        }
        
        let gpt_response = response
            .json::<GptResponse>()
            .await
            .map_err(|e| RustAiToolError::AiModel(format!("Failed to parse GPT response: {}", e)))?;
        
        if gpt_response.choices.is_empty() {
            return Err(RustAiToolError::AiModel("GPT API returned no choices".to_string()));
        }
        
        let content = gpt_response.choices[0].message.content.clone();
        let finish_reason = gpt_response.choices[0].finish_reason.clone();
        
        let usage = gpt_response.usage.map(|u| UsageInfo {
            prompt_tokens: u.prompt_tokens,
            completion_tokens: u.completion_tokens,
            total_tokens: u.total_tokens,
        });
        
        Ok(CompletionResponse {
            content,
            finish_reason,
            usage,
        })
    }
    
    /// Send a completion request to Mistral AI
    ///
    /// # Arguments
    ///
    /// * `request` - Completion request
    ///
    /// # Returns
    ///
    /// The completion response
    async fn send_mistral_request(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        #[derive(Serialize)]
        struct MistralMessage {
            role: String,
            content: String,
        }
        
        #[derive(Serialize)]
        struct MistralRequest {
            model: String,
            messages: Vec<MistralMessage>,
            max_tokens: Option<u32>,
            temperature: Option<f32>,
        }
        
        #[derive(Deserialize)]
        struct MistralResponseChoice {
            message: MistralMessage,
            finish_reason: Option<String>,
        }
        
        #[derive(Deserialize)]
        struct MistralResponseUsage {
            prompt_tokens: u32,
            completion_tokens: u32,
            total_tokens: u32,
        }
        
        #[derive(Deserialize)]
        struct MistralResponse {
            choices: Vec<MistralResponseChoice>,
            usage: Option<MistralResponseUsage>,
        }
        
        let mut messages = Vec::new();
        
        // Add system message if present
        if let Some(system) = request.system {
            messages.push(MistralMessage {
                role: "system".to_string(),
                content: system,
            });
        }
        
        // Add user message
        messages.push(MistralMessage {
            role: "user".to_string(),
            content: request.prompt,
        });
        
        let mistral_request = MistralRequest {
            model: "mistral-large-latest".to_string(), // Use appropriate model version
            messages,
            max_tokens: request.max_tokens,
            temperature: request.temperature,
        };
        
        let api_base = self.config.api_base_url.clone().unwrap_or_else(|| {
            "https://api.mistral.ai/v1/chat/completions".to_string()
        });
        
        let response = self
            .client
            .post(&api_base)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", &self.config.api_key))
            .json(&mistral_request)
            .send()
            .await
            .map_err(|e| RustAiToolError::AiModel(format!("Mistral API request failed: {}", e)))?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(RustAiToolError::AiModel(format!(
                "Mistral API returned error: {}",
                error_text
            )));
        }
        
        let mistral_response = response
            .json::<MistralResponse>()
            .await
            .map_err(|e| RustAiToolError::AiModel(format!("Failed to parse Mistral response: {}", e)))?;
        
        if mistral_response.choices.is_empty() {
            return Err(RustAiToolError::AiModel("Mistral API returned no choices".to_string()));
        }
        
        let content = mistral_response.choices[0].message.content.clone();
        let finish_reason = mistral_response.choices[0].finish_reason.clone();
        
        let usage = mistral_response.usage.map(|u| UsageInfo {
            prompt_tokens: u.prompt_tokens,
            completion_tokens: u.completion_tokens,
            total_tokens: u.total_tokens,
        });
        
        Ok(CompletionResponse {
            content,
            finish_reason,
            usage,
        })
    }
    
    /// Send a completion request to a local model via Ollama
    ///
    /// # Arguments
    ///
    /// * `request` - Completion request
    /// * `model_name` - Local model name
    ///
    /// # Returns
    ///
    /// The completion response
    async fn send_local_request(
        &self,
        request: CompletionRequest,
        model_name: &str,
    ) -> Result<CompletionResponse> {
        #[derive(Serialize)]
        struct OllamaRequest {
            model: String,
            prompt: String,
            system: Option<String>,
            options: Option<OllamaOptions>,
        }
        
        #[derive(Serialize)]
        struct OllamaOptions {
            temperature: Option<f32>,
            num_predict: Option<u32>,
        }
        
        #[derive(Deserialize)]
        struct OllamaResponse {
            response: String,
            done: bool,
        }
        
        let ollama_request = OllamaRequest {
            model: model_name.to_string(),
            prompt: request.prompt,
            system: request.system,
            options: Some(OllamaOptions {
                temperature: request.temperature,
                num_predict: request.max_tokens,
            }),
        };
        
        let api_base = self.config.api_base_url.clone().unwrap_or_else(|| {
            "http://localhost:11434/api/generate".to_string()
        });
        
        let response = self
            .client
            .post(&api_base)
            .header("Content-Type", "application/json")
            .json(&ollama_request)
            .send()
            .await
            .map_err(|e| RustAiToolError::AiModel(format!("Ollama API request failed: {}", e)))?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(RustAiToolError::AiModel(format!(
                "Ollama API returned error: {}",
                error_text
            )));
        }
        
        let ollama_response = response
            .json::<OllamaResponse>()
            .await
            .map_err(|e| RustAiToolError::AiModel(format!("Failed to parse Ollama response: {}", e)))?;
        
        Ok(CompletionResponse {
            content: ollama_response.response,
            finish_reason: Some(if ollama_response.done { "stop".to_string() } else { "length".to_string() }),
            usage: None,
        })
    }
}