# AI Coach CLI

[![CI Status](https://github.com/jpequegn/ai-coach/workflows/CLI%20CI/badge.svg)](https://github.com/jpequegn/ai-coach/actions)
[![codecov](https://codecov.io/gh/jpequegn/ai-coach/branch/main/graph/badge.svg)](https://codecov.io/gh/jpequegn/ai-coach)
[![Crates.io](https://img.shields.io/crates/v/ai-coach-cli.svg)](https://crates.io/crates/ai-coach-cli)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Terminal-based training log application for AI Coach platform. A fast, keyboard-driven interface for logging workouts, tracking goals, and viewing analytics, designed for developer-athletes who prefer CLI tools.

## ✨ Features

- 🏃 **Workout Logging**: Interactive and natural language workout logging
- 📊 **TUI Dashboard**: Beautiful terminal dashboard with real-time metrics
- 🎯 **Goal Tracking**: Create and track fitness goals with progress visualization
- 📈 **Analytics**: Training statistics and insights
- 🔄 **Sync**: Bidirectional sync with AI Coach API
- 📴 **Offline Mode**: Full functionality without internet connection
- ⌨️ **Keyboard-Driven**: Efficient keyboard shortcuts for all operations
- 🎨 **Customizable**: Theme and display preferences

## 📦 Installation

### From Crates.io (Recommended)

```bash
cargo install ai-coach-cli
```

### From Homebrew (macOS/Linux)

```bash
brew tap jpequegn/tap
brew install ai-coach-cli
```

### From GitHub Releases

Download the latest binary for your platform from [GitHub Releases](https://github.com/jpequegn/ai-coach/releases):

```bash
# macOS (ARM64)
curl -L https://github.com/jpequegn/ai-coach/releases/latest/download/ai-coach-macos-arm64.tar.gz | tar xz
chmod +x ai-coach
sudo mv ai-coach /usr/local/bin/

# macOS (Intel)
curl -L https://github.com/jpequegn/ai-coach/releases/latest/download/ai-coach-macos-amd64.tar.gz | tar xz
chmod +x ai-coach
sudo mv ai-coach /usr/local/bin/

# Linux (x86_64)
curl -L https://github.com/jpequegn/ai-coach/releases/latest/download/ai-coach-linux-amd64.tar.gz | tar xz
chmod +x ai-coach
sudo mv ai-coach /usr/local/bin/

# Windows (PowerShell)
Invoke-WebRequest -Uri "https://github.com/jpequegn/ai-coach/releases/latest/download/ai-coach-windows-amd64.zip" -OutFile "ai-coach.zip"
Expand-Archive ai-coach.zip
Move-Item ai-coach/ai-coach.exe C:\Windows\System32\
```

### From Source

```bash
# Clone the repository
git clone https://github.com/jpequegn/ai-coach.git
cd ai-coach

# Build and install the CLI
cargo install --path ai-coach-cli

# Or run directly
cargo run -p ai-coach-cli -- --help
```

## 🚀 Quick Start

### Initial Setup

```bash
# Initialize configuration
ai-coach config init

# Login to AI Coach
ai-coach login
```

### Basic Usage

```bash
# Log a workout (interactive)
ai-coach workout log

# Log a workout (natural language)
ai-coach workout log "Ran 5 miles in 40 minutes"
ai-coach workout log "60 min bike ride at 25km"
ai-coach workout log "Strength training for 45 minutes"

# View recent workouts
ai-coach workout list

# Show training statistics
ai-coach stats

# Launch interactive dashboard
ai-coach dashboard
```

## 📖 Command Reference

### Authentication

| Command | Description |
|---------|-------------|
| `ai-coach login` | Interactive login to AI Coach platform |
| `ai-coach logout` | Clear stored credentials |
| `ai-coach whoami` | Show current authenticated user |

### Workout Management

| Command | Description | Example |
|---------|-------------|---------|
| `ai-coach workout log` | Interactive workout logging | `ai-coach workout log` |
| `ai-coach workout log [TEXT]` | Natural language workout logging | `ai-coach workout log "Ran 5 miles"` |
| `ai-coach workout list` | List recent workouts | `ai-coach workout list --limit 20` |
| `ai-coach workout show <ID>` | Show workout details | `ai-coach workout show abc123` |
| `ai-coach workout edit <ID>` | Edit workout | `ai-coach workout edit abc123` |
| `ai-coach workout delete <ID>` | Delete workout | `ai-coach workout delete abc123 --force` |

#### Natural Language Examples

The workout parser understands natural language descriptions:

```bash
# Running
ai-coach workout log "Ran 5 miles in 40 minutes"
ai-coach workout log "Morning run, 10k in 55 min"
ai-coach workout log "Easy 5k recovery run"

# Cycling
ai-coach workout log "Bike ride 25km in 60 minutes"
ai-coach workout log "Evening cycling session, 45 min"

# Swimming
ai-coach workout log "Swam 2000 meters in 45 minutes"
ai-coach workout log "Pool workout, 30 laps"

# Strength Training
ai-coach workout log "Strength training for 60 minutes"
ai-coach workout log "Upper body workout, 45 min"

# Walking
ai-coach workout log "Walked 3 miles in 50 minutes"
ai-coach workout log "Morning walk, 30 min"
```

### Goal Management

| Command | Description | Example |
|---------|-------------|---------|
| `ai-coach goals list` | List all goals | `ai-coach goals list` |
| `ai-coach goals create` | Create new goal | `ai-coach goals create` |
| `ai-coach goals show <ID>` | Show goal details | `ai-coach goals show goal123` |
| `ai-coach goals update <ID>` | Update goal | `ai-coach goals update goal123` |
| `ai-coach goals complete <ID>` | Mark goal complete | `ai-coach goals complete goal123` |
| `ai-coach goals delete <ID>` | Delete goal | `ai-coach goals delete goal123` |

### Analytics & Dashboard

| Command | Description | Options |
|---------|-------------|---------|
| `ai-coach stats` | Show training statistics | `--period week\|month\|year` |
| `ai-coach dashboard` | Launch TUI dashboard | Interactive mode |

### Sync & Configuration

| Command | Description | Options |
|---------|-------------|---------|
| `ai-coach sync` | Sync with server | `--force` to override conflicts |
| `ai-coach config show` | Show current configuration | |
| `ai-coach config edit` | Edit configuration file | Opens in $EDITOR |
| `ai-coach config init` | Initialize configuration | Creates ~/.ai-coach/config.toml |

### Utility

| Command | Description |
|---------|-------------|
| `ai-coach --version` | Show version info |
| `ai-coach --help` | Show help |
| `ai-coach completions <shell>` | Generate shell completions |

## ⚙️ Configuration

Configuration file location: `~/.ai-coach/config.toml`

### Configuration Options

```toml
[api]
# API endpoint for AI Coach backend
base_url = "http://localhost:3000"
# Request timeout in seconds
timeout_seconds = 30

[auth]
# OAuth tokens (managed automatically)
token = ""
refresh_token = ""

[sync]
# Enable automatic sync after commands
auto_sync = true
# Conflict resolution strategy: "server_wins" | "local_wins" | "manual"
conflict_resolution = "server_wins"
# Sync interval in seconds
sync_interval = 300

[ui]
# UI theme: "dark" | "light"
theme = "dark"
# Date format
date_format = "%Y-%m-%d"
# Time format: "12h" | "24h"
time_format = "24h"
# Show sync status in dashboard
show_sync_status = true

[workouts]
# Default distance unit: "km" | "miles"
default_distance_unit = "km"
# Default duration unit: "minutes" | "hours"
default_duration_unit = "minutes"
# Auto-detect exercise type from description
auto_detect_type = true
```

### Environment Variables

You can override configuration values with environment variables:

```bash
export AI_COACH_API_URL="https://api.ai-coach.example.com"
export AI_COACH_API_TIMEOUT=60
export AI_COACH_THEME="light"
```

## ⌨️ Keyboard Shortcuts

### TUI Dashboard

| Key | Action |
|-----|--------|
| `q` / `Ctrl+C` | Quit dashboard |
| `↑` / `↓` | Navigate up/down |
| `←` / `→` | Switch panels |
| `Tab` | Next panel |
| `Shift+Tab` | Previous panel |
| `Enter` | Select/Activate |
| `r` | Refresh data |
| `s` | Sync with server |
| `n` | New workout |
| `g` | Goals view |
| `h` | Help |
| `/` | Search |
| `?` | Show all shortcuts |

## 🔧 Shell Completions

Generate completions for your shell:

### Bash

```bash
ai-coach completions bash > ~/.bash_completions/ai-coach
echo 'source ~/.bash_completions/ai-coach' >> ~/.bashrc
```

### Zsh

```bash
ai-coach completions zsh > ~/.zfunc/_ai-coach
echo 'fpath=(~/.zfunc $fpath)' >> ~/.zshrc
echo 'autoload -Uz compinit && compinit' >> ~/.zshrc
```

### Fish

```bash
ai-coach completions fish > ~/.config/fish/completions/ai-coach.fish
```

### PowerShell

```powershell
ai-coach completions powershell > ai-coach.ps1
```

## 🧪 Testing

Run the test suite:

```bash
# Run all tests
cargo test -p ai-coach-cli

# Run unit tests only
cargo test -p ai-coach-cli --lib

# Run integration tests
cargo test -p ai-coach-cli --test '*'

# Run specific test
cargo test -p ai-coach-cli test_workout_parser

# Run with coverage
cargo tarpaulin -p ai-coach-cli --out Html
```

## 🐛 Troubleshooting

### Common Issues

#### "Command not found: ai-coach"

**Solution**: Ensure the binary is in your PATH:

```bash
# Add to ~/.bashrc or ~/.zshrc
export PATH="$HOME/.cargo/bin:$PATH"
```

#### "Authentication failed"

**Solution**: Try logging in again:

```bash
ai-coach logout
ai-coach login
```

#### "Cannot connect to server"

**Solution**: Check your configuration and network:

```bash
# Verify configuration
ai-coach config show

# Test network connectivity
curl http://localhost:3000/health

# Update API URL if needed
ai-coach config edit
```

#### "Permission denied" on Linux/macOS

**Solution**: Ensure the binary has execute permissions:

```bash
chmod +x $(which ai-coach)
```

#### Sync conflicts

**Solution**: Choose a conflict resolution strategy:

```bash
# Force sync (server wins)
ai-coach sync --force

# Edit config to change default behavior
ai-coach config edit
# Set: conflict_resolution = "server_wins"
```

### Debug Mode

Enable debug logging:

```bash
export RUST_LOG=debug
ai-coach workout log

# Or for specific module
export RUST_LOG=ai_coach_cli::api=debug
ai-coach sync
```

### Reset Configuration

If you encounter persistent issues:

```bash
# Backup current config
cp ~/.ai-coach/config.toml ~/.ai-coach/config.toml.backup

# Reinitialize
rm -rf ~/.ai-coach
ai-coach config init
ai-coach login
```

## 📚 Examples

### Daily Workout Logging

```bash
# Morning routine
ai-coach workout log "Morning run 5k in 28 minutes"
ai-coach workout log "Strength training upper body 45 min"

# Evening session
ai-coach workout log "Evening bike ride 25km easy pace"
```

### Goal Tracking

```bash
# Create a goal
ai-coach goals create

# Track progress
ai-coach goals list
ai-coach stats --period month

# Complete when achieved
ai-coach goals complete goal-id
```

### Offline Mode

```bash
# Log workouts offline
ai-coach workout log "Ran 10k in 52 minutes"
ai-coach workout log "Strength training 60 min"

# Later, sync when online
ai-coach sync
```

## 🤝 Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](../CONTRIBUTING.md) for guidelines.

### Development Setup

```bash
# Clone repository
git clone https://github.com/jpequegn/ai-coach.git
cd ai-coach

# Run in development
cargo run -p ai-coach-cli -- --help

# Run tests
cargo test -p ai-coach-cli

# Format code
cargo fmt -p ai-coach-cli

# Lint code
cargo clippy -p ai-coach-cli
```

## 📝 License

MIT License - see [LICENSE](../LICENSE) for details.

## 🔗 Links

- [GitHub Repository](https://github.com/jpequegn/ai-coach)
- [Issue Tracker](https://github.com/jpequegn/ai-coach/issues)
- [API Documentation](https://github.com/jpequegn/ai-coach/tree/main/ai-coach-api)
- [Project Architecture](../CLAUDE.md)

## 💬 Support

- **Issues**: [GitHub Issues](https://github.com/jpequegn/ai-coach/issues)
- **Discussions**: [GitHub Discussions](https://github.com/jpequegn/ai-coach/discussions)

---

**Made with ❤️ by the AI Coach Team**
