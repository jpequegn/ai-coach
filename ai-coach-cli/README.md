# AI Coach CLI

Terminal-based training log application for AI Coach platform. A fast, keyboard-driven interface for logging workouts, tracking goals, and viewing analytics, designed for developer-athletes who prefer CLI tools.

## Status

**Phase 1 Complete** - CLI Foundation (Week 1-2)
- ✅ Cargo workspace structure
- ✅ CLI argument parsing with clap
- ✅ Configuration system (~/.ai-coach/config.toml)
- ✅ Command structure for all major features
- ✅ Shell completion generation

## Installation

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

## Quick Start

```bash
# Initialize configuration
ai-coach config init

# Login to AI Coach
ai-coach login

# Log a workout (interactive)
ai-coach workout log

# Show training statistics
ai-coach stats

# Launch interactive dashboard
ai-coach dashboard
```

## Commands

### Authentication
- `ai-coach login` - Interactive login
- `ai-coach logout` - Clear credentials
- `ai-coach whoami` - Show current user (coming soon)

### Workout Management
- `ai-coach workout log` - Interactive workout logging
- `ai-coach workout log "Ran 5 miles"` - Natural language log (coming soon)
- `ai-coach workout list` - List recent workouts (coming soon)
- `ai-coach workout show <id>` - Show workout details (coming soon)
- `ai-coach workout edit <id>` - Edit workout (coming soon)
- `ai-coach workout delete <id>` - Delete workout (coming soon)

### Goals & Planning
- `ai-coach goals list` - List all goals (coming soon)
- `ai-coach goals create` - Create new goal (coming soon)
- `ai-coach goals update <id>` - Update goal (coming soon)
- `ai-coach goals complete <id>` - Mark goal complete (coming soon)

### Analytics & Dashboard
- `ai-coach stats` - Show training statistics (coming soon)
- `ai-coach dashboard` - Launch TUI dashboard (coming soon)

### Sync & Configuration
- `ai-coach sync` - Sync with server (coming soon)
- `ai-coach config show` - Show current configuration
- `ai-coach config edit` - Edit configuration file
- `ai-coach config init` - Initialize configuration

### Utility
- `ai-coach --version` - Show version info
- `ai-coach --help` - Show help
- `ai-coach completions <shell>` - Generate shell completions

## Configuration

Configuration file location: `~/.ai-coach/config.toml`

```toml
[api]
base_url = "http://localhost:3000"
timeout_seconds = 30

[auth]
token = ""
refresh_token = ""

[sync]
auto_sync = true
conflict_resolution = "server_wins"

[ui]
theme = "dark"
date_format = "%Y-%m-%d"
time_format = "24h"
show_sync_status = true

[workouts]
default_distance_unit = "km"
default_duration_unit = "minutes"
```

## Shell Completions

Generate completions for your shell:

```bash
# Bash
ai-coach completions bash > ~/.bash_completions/ai-coach

# Zsh
ai-coach completions zsh > ~/.zfunc/_ai-coach

# Fish
ai-coach completions fish > ~/.config/fish/completions/ai-coach.fish
```

## Development Roadmap

### Phase 1: CLI Foundation ✅
- [x] Project setup and workspace structure
- [x] CLI argument parsing
- [x] Configuration system
- [x] Command structure

### Phase 2: Authentication (Week 2)
- [ ] Login command with API integration
- [ ] JWT token storage
- [ ] Token refresh logic
- [ ] Logout command

### Phase 3: Workout Logging (Week 3-4)
- [ ] Interactive workout logging
- [ ] Natural language parsing
- [ ] Workout list view
- [ ] Workout details/edit/delete

### Phase 4: TUI Dashboard (Week 5-6)
- [ ] Interactive dashboard with ratatui
- [ ] Multi-panel layout
- [ ] ASCII charts for training volume
- [ ] Keyboard navigation

### Phase 5: Goals & Planning (Week 7)
- [ ] Goal management commands
- [ ] Training plan viewer
- [ ] Progress tracking

### Phase 6: Sync & Offline Mode (Week 8)
- [ ] Sync engine with conflict resolution
- [ ] Auto-sync on command completion
- [ ] Offline queue management

### Phase 7: Analytics & Insights (Week 9)
- [ ] Stats command with metrics
- [ ] AI insights from API
- [ ] Training consistency tracking

### Phase 8: Testing & Distribution (Week 10-11)
- [ ] Comprehensive testing
- [ ] Documentation
- [ ] GitHub releases
- [ ] Publish to crates.io
- [ ] Homebrew formula

## Contributing

See [CLAUDE.md](../CLAUDE.md) for development guidelines and project architecture.

## License

MIT
