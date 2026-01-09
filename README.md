# Perplexity AI Chat

An interactive Rust-based chat application for Perplexity AI with chat history, model management, customizable directives, and automatic code generation including Kubernetes and Docker configurations.

## Features

- **Interactive Chat**: Real-time conversation with Perplexity AI models
- **Chat History**: Maintains conversation context throughout the session
- **Model Management**: Switch between different Perplexity AI models
- **Custom Directives**: Set default behavior for all conversations
- **Command System**: Easy-to-use commands for managing the chat session
- **Automatic Code Generation**: Detects code requests and saves files to `interact/` folder
- **Kubernetes Support**: Specialized detection and naming for K8s resources (Deployments, Services, ConfigMaps, etc.)
- **Docker Support**: Automatic Dockerfile and Docker Compose generation with proper naming

## Setup

1. **Set your API key**:
   ```bash
   export PERPLEXITY_API_KEY="your_api_key_here"
   ```

2. **Optional: Set a default directive** (otherwise a default will be used):
   ```bash
   export DEFAULT_DIRECTIVE="You are a helpful AI assistant. Be concise and accurate."
   ```

3. **Build and run**:
   ```bash
   cargo build --release
   cargo run --release
   ```

## Commands

All commands start with `/`:

- `/exit` - Exit the chat application
- `/clear` - Clear chat history (keeps the system directive)
- `/showhistory` - Display the complete chat history
- `/showdirective` - Show the current system directive
- `/models` - List all available Perplexity AI models
- `/changemodel <model>` - Switch to a different AI model
- `/setdirective <directive>` - Update the default directive for all chats
- `/help` - Show available commands and models

## Available Models

- `sonar` (default)
- `sonar-pro`
- `sonar-reasoning`
- `llama-3.1-sonar-small-128k-online`
- `llama-3.1-sonar-large-128k-online`
- `llama-3.1-sonar-huge-128k-online`

## Usage Examples

### Basic Chat
```
You: What is Rust programming language?
AI: Rust is a systems programming language focused on safety, speed, and concurrency...
```

### Changing Models
```
You: /changemodel sonar-pro
Model changed to: sonar-pro
```

### Setting Custom Directive
```
You: /setdirective You are an expert software engineer. Provide detailed technical answers.
Default directive updated: You are an expert software engineer. Provide detailed technical answers.
```

### Kubernetes Resource Generation
```
You: Create a Kubernetes deployment for a Node.js application
AI: ```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: nodejs-app
spec:
  replicas: 3
  selector:
    matchLabels:
      app: nodejs-app
  template:
    metadata:
      labels:
        app: nodejs-app
    spec:
      containers:
      - name: nodejs-app
        image: node:16-alpine
        ports:
        - containerPort: 3000
```

Code saved to: interact/deployment_a1b2c3d4.yaml
Files saved to interact/ folder:
  deployment_a1b2c3d4.yaml
```

### Docker Configuration Generation
```
You: Create a Dockerfile for a Python Flask application
AI: ```dockerfile
FROM python:3.9-slim

WORKDIR /app

COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

COPY . .

EXPOSE 5000

CMD ["python", "app.py"]
```

Code saved to: interact/Dockerfile
Files saved to interact/ folder:
  Dockerfile
```

### Docker Compose Generation
```
You: Create a docker-compose setup for a web app with database
AI: ```yaml
version: '3.8'

services:
  web:
    build: .
    ports:
      - "3000:3000"
    depends_on:
      - db
    environment:
      - DATABASE_URL=postgresql://user:password@db:5432/myapp
  
  db:
    image: postgres:13
    environment:
      - POSTGRES_DB=myapp
      - POSTGRES_USER=user
      - POSTGRES_PASSWORD=password
    volumes:
      - postgres_data:/var/lib/postgresql/data

volumes:
  postgres_data:
```

Code saved to: interact/docker-compose_b5c6d7e8.yaml
Files saved to interact/ folder:
  docker-compose_b5c6d7e8.yaml
```

### Clearing History
```
You: /clear
Chat history cleared.
```

### Showing History
```
You: /showhistory
Chat History:
  [1] You: What is Rust programming language?
  [2] AI: Rust is a systems programming language focused on safety, speed, and concurrency...
  [3] You: What are its main features?
  [4] AI: The main features of Rust include...
```

### Showing Current Directive
```
You: /showdirective
Current Directive:
  You are a helpful AI assistant. Be concise and accurate.
```

## Architecture

The application is built with:
- **Tokio** for async runtime
- **Reqwest** for HTTP client
- **Serde** for JSON serialization/deserialization
- **Colored** for terminal output formatting
- **Clap** for command parsing
- **Regex** for code pattern detection
- **UUID** for unique file naming

## Code Generation Features

### Supported Languages and File Types
- **Programming Languages**: Rust, Python, JavaScript, TypeScript, Go, Java, C/C++, PHP, Ruby, Swift, Kotlin, Scala, R, Dart, Lua, Perl
- **Web Technologies**: HTML, CSS, JSON, YAML, Markdown
- **Shell Scripts**: Bash, Shell
- **Database**: SQL
- **Container Technologies**: Dockerfile, Docker Compose
- **Orchestration**: Kubernetes resources (Deployments, Services, ConfigMaps, Secrets, Ingress, StatefulSets, DaemonSets, Jobs, CronJobs)

### Smart File Naming
- **Kubernetes Resources**: Automatically detected and named based on resource type (e.g., `deployment_a1b2c3d4.yaml`, `service_b5c6d7e8.yaml`)
- **Docker Files**: `Dockerfile` for Dockerfiles, `docker-compose_a1b2c3d4.yaml` for Docker Compose
- **General Code**: `generated_a1b2c3d4.{ext}` with appropriate file extensions

### Detection Patterns
The application automatically detects code generation requests for:
- General code writing and scripting
- Kubernetes resource creation and configuration
- Docker containerization and multi-service setups
- Infrastructure as code and deployment configurations

## Error Handling

The application includes comprehensive error handling for:
- Missing API keys
- Network failures
- Invalid model names
- API response errors
- File system operations for code generation

All errors are displayed with clear, colored messages to help with troubleshooting.