use std::env;
use std::io::{self, Write};
use std::fs;
use std::path::Path;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use colored::*;
use regex::Regex;

#[derive(Serialize, Deserialize, Clone)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
}

#[derive(Serialize, Deserialize, Clone)]
struct Message {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: MessageResponse,
}

#[derive(Deserialize)]
struct MessageResponse {
    #[allow(dead_code)]
    role: String,
    content: String,
}

#[derive(Clone)]
struct ChatSession {
    history: Vec<Message>,
    model: String,
    default_directive: String,
}

impl ChatSession {
    fn new() -> Self {
        let default_directive = env::var("DEFAULT_DIRECTIVE")
            .unwrap_or_else(|_| "You are a helpful AI assistant. Be concise and accurate. When asked to write code, scripts, or create files, provide ONLY the code in appropriate code blocks with language specification. Do not include explanations unless specifically asked. For Kubernetes and Docker setups, provide complete, production-ready configurations with proper naming conventions.".to_string());
        
        let mut session = ChatSession {
            history: Vec::new(),
            model: "sonar".to_string(),
            default_directive,
        };
        
        // Add system message with default directive
        session.add_message("system".to_string(), session.default_directive.clone());
        session
    }
    
    fn add_message(&mut self, role: String, content: String) {
        self.history.push(Message { role, content });
    }
    
    fn clear_history(&mut self) {
        self.history.clear();
        // Re-add the system message with default directive
        self.add_message("system".to_string(), self.default_directive.clone());
    }
    
    fn set_model(&mut self, model: String) {
        self.model = model;
    }
    
    fn set_default_directive(&mut self, directive: String) {
        self.default_directive = directive.clone();
        // Update the system message if it exists
        if let Some(system_msg) = self.history.get_mut(0) {
            if system_msg.role == "system" {
                system_msg.content = directive;
            }
        }
    }
}

const AVAILABLE_MODELS: &[&str] = &[
    "sonar",
    "sonar-pro",
    "sonar-reasoning",
    "llama-3.1-sonar-small-128k-online",
    "llama-3.1-sonar-large-128k-online",
    "llama-3.1-sonar-huge-128k-online",
];

fn print_help() {
    println!("{}", "Available commands:".green().bold());
    println!("  {} - Exit the chat application", "exit".cyan());
    println!("  {} - Clear chat history", "clear".cyan());
    println!("  {} - Show chat history", "showhistory".cyan());
    println!("  {} - Show current directive", "showdirective".cyan());
    println!("  {} - List available models", "models".cyan());
    println!("  {} <model> - Change the AI model", "changemodel".cyan());
    println!("  {} <directive> - Set default directive for all chats", "setdirective".cyan());
    println!("  {} - Show this help message", "help".cyan());
    println!();
    println!("{}", "Available models:".green().bold());
    for model in AVAILABLE_MODELS {
        println!("  {}", model.yellow());
    }
    println!();
}

fn print_welcome() {
    println!("{}", "╔══════════════════════════════════════════════════════════════╗".cyan());
    println!("{}", "║                    Perplexity AI Chat                      ║".cyan());
    println!("{}", "╚══════════════════════════════════════════════════════════════╝".cyan());
    println!();
    println!("Type {} to see available commands", "help".cyan());
    println!("Type {} to exit the application", "exit".cyan());
    println!();
}

async fn send_chat_request(
    client: &Client,
    api_key: &str,
    session: &ChatSession,
) -> Result<String, Box<dyn std::error::Error>> {
    let req_body = ChatRequest {
        model: session.model.clone(),
        messages: session.history.clone(),
    };

    let res = client
        .post("https://api.perplexity.ai/chat/completions")
        .bearer_auth(api_key)
        .json(&req_body)
        .send()
        .await?;

    if !res.status().is_success() {
        let status = res.status();
        let text = res.text().await?;
        return Err(format!("Request failed: {} - {}", status, text).into());
    }

    let chat_res: ChatResponse = res.json().await?;
    if let Some(first) = chat_res.choices.first() {
        Ok(first.message.content.clone())
    } else {
        Err("No choices in response".into())
    }
}

fn parse_command(input: &str) -> Option<(&str, Option<&str>)> {
    let trimmed = input.trim();
    if !trimmed.starts_with('/') {
        return None;
    }
    
    let parts: Vec<&str> = trimmed[1..].splitn(2, ' ').collect();
    match parts.len() {
        1 => Some((parts[0], None)),
        2 => Some((parts[0], Some(parts[1]))),
        _ => None,
    }
}

fn extract_and_save_code_blocks(response: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut saved_files = Vec::new();
    
    // Regex to match code blocks with language specification (more permissive)
    let code_block_regex = Regex::new(r"```(\w+)?\s*\n([\s\S]*?)\n```")?;
    
    for capture in code_block_regex.captures_iter(response) {
        let language = capture.get(1).map(|m| m.as_str()).unwrap_or("txt");
        let mut code = capture.get(2).unwrap().as_str().to_string();
        
        // Clean up the code - remove leading/trailing whitespace
        code = code.trim().to_string();
        
        // Debug output
        println!("{} {}", "Detected code block:".yellow(), language);
        println!("{} {}", "Code length:".yellow(), code.len());
        
        // Handle different file types
        match language {
            "yaml" | "yml" => {
                if code.contains("apiVersion:") || code.contains("kind:") || code.contains("metadata:") {
                    // Handle Kubernetes multi-document YAML files
                    let k8s_files = split_k8s_resources(&code)?;
                    saved_files.extend(k8s_files);
                } else if code.contains("version:") && code.contains("services:") {
                    // Handle Docker Compose
                    let filename = format!("docker-compose_{}.yaml",
                        uuid::Uuid::new_v4().to_string()[..8].to_string());
                    let filepath = Path::new("interact").join(&filename);
                    
                    ensure_directory_exists("interact")?;
                    fs::write(&filepath, code)?;
                    saved_files.push(filename);
                    println!("{} {}", "Code saved to:".green(), filepath.display().to_string().yellow());
                } else {
                    // Generic YAML file
                    let filename = format!("generated_{}.yaml",
                        uuid::Uuid::new_v4().to_string()[..8].to_string());
                    let filepath = Path::new("interact").join(&filename);
                    
                    ensure_directory_exists("interact")?;
                    fs::write(&filepath, code)?;
                    saved_files.push(filename);
                    println!("{} {}", "Code saved to:".green(), filepath.display().to_string().yellow());
                }
            },
            "dockerfile" => {
                let filepath = Path::new("interact").join("Dockerfile");
                ensure_directory_exists("interact")?;
                fs::write(&filepath, code)?;
                saved_files.push("Dockerfile".to_string());
                println!("{} {}", "Code saved to:".green(), filepath.display().to_string().yellow());
            },
            _ => {
                // Handle other file types
                let extension = match language {
                    "rust" => "rs",
                    "javascript" | "js" => "js",
                    "typescript" | "ts" => "ts",
                    "python" | "py" => "py",
                    "html" => "html",
                    "css" => "css",
                    "json" => "json",
                    "markdown" | "md" => "md",
                    "bash" | "sh" => "sh",
                    "sql" => "sql",
                    "go" => "go",
                    "java" => "java",
                    "cpp" | "c++" => "cpp",
                    "c" => "c",
                    "php" => "php",
                    "rb" => "rb",
                    "swift" => "swift",
                    "kotlin" => "kt",
                    "scala" => "scala",
                    "r" => "r",
                    "dart" => "dart",
                    "lua" => "lua",
                    "perl" => "pl",
                    _ => "txt",
                };
                
                let filename = format!("generated_{}.{}",
                    uuid::Uuid::new_v4().to_string()[..8].to_string(),
                    extension);
                let filepath = Path::new("interact").join(&filename);
                
                ensure_directory_exists("interact")?;
                fs::write(&filepath, code)?;
                saved_files.push(filename);
                println!("{} {}", "Code saved to:".green(), filepath.display().to_string().yellow());
            }
        }
    }
    
    Ok(saved_files)
}

fn ensure_directory_exists(dir_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    if !Path::new(dir_path).exists() {
        fs::create_dir(dir_path)?;
    }
    Ok(())
}

fn split_k8s_resources(yaml_content: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut saved_files = Vec::new();
    
    // Split YAML content by document separator (---)
    let documents: Vec<&str> = yaml_content.split("---")
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    
    // Create a session-specific folder
    let session_id = uuid::Uuid::new_v4().to_string()[..8].to_string();
    let session_dir = Path::new("interact").join(format!("k8s_{}", session_id));
    ensure_directory_exists(session_dir.to_str().unwrap())?;
    
    println!("{} {}", "Kubernetes resources saved to:".green(), session_dir.display().to_string().yellow());
    
    for (index, document) in documents.iter().enumerate() {
        // Debug output
        println!("{} {}", "Processing document:".yellow(), index + 1);
        println!("{} {}", "Content preview:".yellow(), &document[..document.len().min(50)]);
        
        // Extract resource kind and metadata name
        let resource_info = extract_k8s_resource_info(document);
        
        let filename = if let (Some(kind), Some(name)) = resource_info {
            format!("{}-{}.yaml",
                kind.to_lowercase(),
                name.to_lowercase().replace('_', "-"))
        } else {
            format!("resource-{}.yaml", index + 1)
        };
        
        let filepath = session_dir.join(&filename);
        fs::write(&filepath, document)?;
        saved_files.push(format!("k8s_{}/{}", session_id, filename));
        
        println!("  {}", filepath.display().to_string().cyan());
    }
    
    Ok(saved_files)
}

fn extract_k8s_resource_info(yaml_doc: &str) -> (Option<String>, Option<String>) {
    let mut kind = None;
    let mut name = None;
    let mut in_metadata = false;
    
    for line in yaml_doc.lines() {
        let line = line.trim();
        
        if line.starts_with("kind:") {
            kind = Some(line.split(':').nth(1).unwrap_or("").trim().to_string());
        } else if line.starts_with("metadata:") {
            in_metadata = true;
        } else if in_metadata && line.starts_with("name:") {
            name = Some(line.split(':').nth(1).unwrap_or("").trim().to_string());
            break;
        } else if line.starts_with("name:") && kind.is_some() {
            // Fallback if name is not under metadata
            name = Some(line.split(':').nth(1).unwrap_or("").trim().to_string());
            break;
        }
    }
    
    (kind, name)
}

fn is_code_generation_request(user_input: &str) -> bool {
    let patterns = [
        r"(?i)write.*code",
        r"(?i)create.*script",
        r"(?i)generate.*code",
        r"(?i)implement.*function",
        r"(?i)write.*rust",
        r"(?i)write.*python",
        r"(?i)write.*javascript",
        r"(?i)write.*typescript",
        r"(?i)write.*java",
        r"(?i)write.*go",
        r"(?i)write.*c\+\+",
        r"(?i)write.*c#",
        r"(?i)write.*php",
        r"(?i)write.*html",
        r"(?i)write.*css",
        r"(?i)write.*sql",
        r"(?i)create.*file",
        r"(?i)generate.*file",
        r"(?i)build.*script",
        r"(?i)develop.*function",
        r"(?i)code.*to.*do",
        r"(?i)script.*to.*do",
        // Kubernetes patterns
        r"(?i)create.*kubernetes",
        r"(?i)create.*k8s",
        r"(?i)kubernetes.*deployment",
        r"(?i)k8s.*deployment",
        r"(?i)kubernetes.*service",
        r"(?i)k8s.*service",
        r"(?i)kubernetes.*pod",
        r"(?i)k8s.*pod",
        r"(?i)kubernetes.*configmap",
        r"(?i)k8s.*configmap",
        r"(?i)kubernetes.*secret",
        r"(?i)k8s.*secret",
        r"(?i)kubernetes.*ingress",
        r"(?i)k8s.*ingress",
        r"(?i)kubernetes.*yaml",
        r"(?i)k8s.*yaml",
        r"(?i)deployment.*yaml",
        r"(?i)service.*yaml",
        r"(?i)pod.*yaml",
        r"(?i)configmap.*yaml",
        r"(?i)secret.*yaml",
        r"(?i)ingress.*yaml",
        r"(?i)namespace.*yaml",
        r"(?i)statefulset.*yaml",
        r"(?i)persistentvolume.*yaml",
        r"(?i)pvc.*yaml",
        r"(?i)wordpress.*kubernetes",
        r"(?i)wordpress.*k8s",
        r"(?i)mysql.*kubernetes",
        r"(?i)mysql.*k8s",
        // Docker patterns
        r"(?i)create.*docker",
        r"(?i)docker.*file",
        r"(?i)dockerfile",
        r"(?i)docker.*compose",
        r"(?i)docker.*container",
        r"(?i)docker.*image",
        r"(?i)docker.*setup",
        r"(?i)docker.*config",
        r"(?i)docker.*deployment",
        r"(?i)container.*setup",
        r"(?i)container.*config",
        // Infrastructure patterns
        r"(?i)create.*infrastructure",
        r"(?i)setup.*infrastructure",
        r"(?i)deploy.*kubernetes",
        r"(?i)deploy.*docker",
        r"(?i)container.*orchestration",
        r"(?i)microservices.*setup",
        r"(?i)service.*mesh",
        r"(?i)helm.*chart",
        r"(?i)kustomize",
    ];
    
    for pattern in &patterns {
        if let Ok(regex) = Regex::new(pattern) {
            if regex.is_match(user_input) {
                return true;
            }
        }
    }
    false
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get API key from env
    let api_key = env::var("PERPLEXITY_API_KEY")
        .expect("PERPLEXITY_API_KEY env var must be set");

    // Build HTTP client
    let client = Client::new();
    
    // Initialize chat session
    let mut session = ChatSession::new();
    
    print_welcome();
    
    loop {
        print!("{}", "You: ".green().bold());
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();
        
        if input.is_empty() {
            continue;
        }
        
        // Check for commands
        if let Some((command, arg)) = parse_command(input) {
            match command {
                "exit" => {
                    println!("{}", "Goodbye!".cyan());
                    break;
                }
                "clear" => {
                    session.clear_history();
                    println!("{}", "Chat history cleared.".green());
                    continue;
                }
                "models" => {
                    println!("{}", "Available models:".green().bold());
                    for model in AVAILABLE_MODELS {
                        let current = if model == &session.model { " (current)" } else { "" };
                        println!("  {}{}", model.yellow(), current.green());
                    }
                    continue;
                }
                "changemodel" => {
                    if let Some(model) = arg {
                        if AVAILABLE_MODELS.contains(&model) {
                            session.set_model(model.to_string());
                            println!("{} {}", "Model changed to:".green(), model.yellow());
                        } else {
                            println!("{} {}", "Error:".red(), "Invalid model. Use /models to see available options.".red());
                        }
                    } else {
                        println!("{} {}", "Error:".red(), "Please specify a model. Usage: /changemodel <model>".red());
                    }
                    continue;
                }
                "setdirective" => {
                    if let Some(directive) = arg {
                        session.set_default_directive(directive.to_string());
                        println!("{} {}", "Default directive updated:".green(), directive.yellow());
                    } else {
                        println!("{} {}", "Error:".red(), "Please specify a directive. Usage: /setdirective <directive>".red());
                    }
                    continue;
                }
                "showhistory" => {
                    println!("{}", "Chat History:".green().bold());
                    if session.history.is_empty() || (session.history.len() == 1 && session.history[0].role == "system") {
                        println!("  {}", "No chat history yet.".yellow());
                    } else {
                        for (i, message) in session.history.iter().enumerate() {
                            if message.role == "system" {
                                continue; // Skip system messages in history display
                            }
                            let role_color = if message.role == "user" {
                                "You:".green().bold().to_string()
                            } else {
                                "AI:".blue().bold().to_string()
                            };
                            println!("  [{}] {}: {}", i + 1, role_color, message.content);
                        }
                    }
                    continue;
                }
                "showdirective" => {
                    println!("{}", "Current Directive:".green().bold());
                    println!("  {}", session.default_directive.yellow());
                    continue;
                }
                "help" => {
                    print_help();
                    continue;
                }
                _ => {
                    println!("{} {}", "Error:".red(), "Unknown command. Type /help for available commands.".red());
                    continue;
                }
            }
        }
        
        // Regular chat message
        session.add_message("user".to_string(), input.to_string());
        
        print!("{} ", "AI:".blue().bold());
        io::stdout().flush()?;
        
        match send_chat_request(&client, &api_key, &session).await {
            Ok(response) => {
                println!("{}", response);
                session.add_message("assistant".to_string(), response.clone());
                
                // Check if this was a code generation request and extract code blocks
                if is_code_generation_request(input) {
                    match extract_and_save_code_blocks(&response) {
                        Ok(saved_files) => {
                            if !saved_files.is_empty() {
                                println!();
                                println!("{}", "Files saved to interact/ folder:".green().bold());
                                for file in saved_files {
                                    println!("  {}", file.yellow());
                                }
                            }
                        }
                        Err(e) => {
                            println!("{} {}", "Error saving code files:".red(), e);
                        }
                    }
                }
            }
            Err(e) => {
                println!("{} {}", "Error:".red(), e);
            }
        }
        
        println!();
    }
    
    Ok(())
}
