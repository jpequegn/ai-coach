# Feature #8: CLI Tool for Athletes

## Overview
Build a terminal-based training log application in Rust that provides a fast, keyboard-driven interface for logging workouts, tracking goals, and viewing analytics. Designed for developer-athletes who prefer CLI tools over web/mobile interfaces.

## Business Value
- **Target Niche**: Appeals to technical users and developers
- **Workflow Integration**: Integrates into developer workflows
- **Offline-First**: Works without internet, syncs when available
- **Brand Building**: Open-source CLI showcases platform capabilities
- **Adoption**: Gateway for technical community to discover AI Coach

## Technical Architecture

### Components
1. **CLI Framework** (`clap` for arg parsing)
2. **TUI Interface** (`ratatui` for terminal UI)
3. **Local Storage** (SQLite for offline data)
4. **API Client** (REST client for sync)
5. **Configuration** (TOML config file)

### Technology Stack
- `clap` v4 - CLI argument parsing with derive macros
- `ratatui` - Terminal UI framework
- `crossterm` - Cross-platform terminal manipulation
- `reqwest` - HTTP client for API calls
- `serde` - Serialization for config and data
- `rusqlite` - Local SQLite database
- `tokio` - Async runtime
- `chrono` - Date/time handling
- `dialoguer` - Interactive prompts
- `indicatif` - Progress bars and spinners

## Implementation Tasks

### Phase 1: CLI Foundation (Week 1-2)
**Task 1.1: Project Setup**
- Create new Cargo workspace: `ai-coach-cli`
- Set up project structure:
  ```
  ai-coach-cli/
  ├── src/
  │   ├── main.rs
  │   ├── commands/
  │   ├── api/
  │   ├── storage/
  │   ├── ui/
  │   └── config/
  ├── Cargo.toml
  └── README.md
  ```
- Configure dependencies in Cargo.toml
- Set up CI/CD for releases (GitHub Actions)

**Task 1.2: CLI Argument Parsing**
- Define command structure using `clap`:
  ```
  ai-coach login
  ai-coach workout log
  ai-coach workout list
  ai-coach goals list
  ai-coach stats
  ai-coach sync
  ```
- Implement subcommands with derive macros
- Add global flags: `--offline`, `--verbose`, `--config`
- Create help documentation
- Add shell completion generation (bash/zsh/fish)

**Task 1.3: Configuration System**
- Create `~/.ai-coach/config.toml` for settings
- Implement config struct with defaults:
  ```toml
  [api]
  base_url = "https://api.ai-coach.app"
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
  ```
- Add config validation
- Implement config init/edit commands

**Task 1.4: Local Storage**
- Set up SQLite database at `~/.ai-coach/local.db`
- Create schema for offline storage:
  ```sql
  CREATE TABLE workouts (
      id TEXT PRIMARY KEY,
      date TEXT NOT NULL,
      exercise_type TEXT NOT NULL,
      duration_minutes INTEGER,
      distance_km REAL,
      notes TEXT,
      synced INTEGER DEFAULT 0,
      created_at TEXT DEFAULT CURRENT_TIMESTAMP
  );

  CREATE TABLE goals (
      id TEXT PRIMARY KEY,
      title TEXT NOT NULL,
      target_date TEXT,
      status TEXT,
      synced INTEGER DEFAULT 0
  );

  CREATE TABLE sync_queue (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      entity_type TEXT NOT NULL,
      entity_id TEXT NOT NULL,
      operation TEXT NOT NULL, -- create, update, delete
      data TEXT NOT NULL,
      created_at TEXT DEFAULT CURRENT_TIMESTAMP
  );
  ```
- Implement storage layer with CRUD operations
- Add migration system for schema updates

### Phase 2: Authentication (Week 2)
**Task 2.1: Login Command**
- Implement `ai-coach login` command
- Interactive username/password prompt using `dialoguer`
- Call `/api/v1/auth/login` endpoint
- Store JWT tokens securely in config file
- Add token refresh logic
- Implement `ai-coach logout` command

**Task 2.2: API Client**
- Create `ApiClient` struct with `reqwest`
- Implement authentication headers
- Add automatic token refresh on 401
- Handle offline mode gracefully
- Implement retry logic with exponential backoff
- Add request/response logging in verbose mode

### Phase 3: Workout Logging (Week 3-4)
**Task 3.1: Quick Log Command**
- Implement `ai-coach workout log` with interactive prompts:
  ```
  $ ai-coach workout log
  ? Exercise type: Running
  ? Duration (minutes): 45
  ? Distance (km): 8.5
  ? Notes: Easy recovery run
  ✓ Workout logged successfully!
  ```
- Support inline args: `ai-coach workout log --type running --duration 45`
- Add preset templates for common workouts
- Validate input data
- Store locally and queue for sync

**Task 3.2: Natural Language Parsing**
- Parse workout descriptions: `ai-coach workout log "Ran 5 miles in 40 minutes"`
- Extract: exercise type, duration, distance, intensity
- Use regex patterns for common formats
- Support multiple units (km/miles, min/hours)
- Fall back to interactive prompts if parsing fails

**Task 3.3: Workout List View**
- Implement `ai-coach workout list` command
- Display recent workouts in table format:
  ```
  Date       | Type    | Duration | Distance | Notes
  -----------+---------+----------+----------+-------
  2025-09-30 | Running | 45 min   | 8.5 km   | Easy recovery
  2025-09-29 | Cycling | 60 min   | 25 km    | Intervals
  ```
- Add filters: `--type`, `--from-date`, `--to-date`
- Implement pagination for long lists
- Show sync status icon (✓ synced, ⏳ pending)

**Task 3.4: Workout Details**
- Implement `ai-coach workout show <id>` command
- Display detailed workout information
- Include AI recommendations if available
- Show related training metrics
- Add edit and delete options

### Phase 4: TUI Dashboard (Week 5-6)
**Task 4.1: Interactive Dashboard**
- Implement `ai-coach dashboard` command using `ratatui`
- Create layout with multiple panels:
  - Top: Current week summary
  - Left: Recent workouts
  - Right: Upcoming goals
  - Bottom: Quick actions
- Add keyboard navigation (vim-style: hjkl)
- Implement real-time updates

**Task 4.2: Stats Visualization**
- Create ASCII charts for training volume
- Weekly/monthly training summary
- Progress toward goals
- Personal records
- Training load graph
- Use `ratatui` widgets: Block, Chart, Gauge, List

**Task 4.3: Interactive Features**
- Quick workout log from dashboard
- Navigate between views (workouts, goals, stats)
- Filter and search functionality
- Sync status indicator
- Help overlay (press '?')

### Phase 5: Goals & Planning (Week 7)
**Task 5.1: Goal Management**
- Implement `ai-coach goals list` command
- `ai-coach goals create` with interactive prompts
- `ai-coach goals update <id>` to modify goals
- `ai-coach goals complete <id>` to mark done
- Track progress percentage
- Show days remaining

**Task 5.2: Training Plans**
- Implement `ai-coach plan show` to view current plan
- Display weekly training structure
- Show planned vs actual workouts
- Highlight missed sessions
- Add notes and adjustments

### Phase 6: Sync & Offline Mode (Week 8)
**Task 6.1: Sync Engine**
- Implement `ai-coach sync` command
- Upload pending workouts from sync queue
- Download latest data from server
- Resolve conflicts (last-write-wins by default)
- Show sync progress with spinner
- Add `--dry-run` flag to preview changes

**Task 6.2: Auto-Sync**
- Implement background sync on command completion
- Configurable via `sync.auto_sync` in config
- Only sync if online and not explicitly offline mode
- Silent background sync with optional notifications

**Task 6.3: Conflict Resolution**
- Detect local vs server conflicts
- Implement resolution strategies:
  - `server_wins`: Server data takes precedence
  - `local_wins`: Local data takes precedence
  - `manual`: Prompt user to choose
- Store conflict history
- Add `ai-coach sync --resolve` for manual resolution

### Phase 7: Analytics & Insights (Week 9)
**Task 7.1: Stats Command**
- Implement `ai-coach stats` command
- Show summary statistics:
  - Total workouts this week/month/year
  - Total distance/duration
  - Average pace/heart rate
  - Personal records
  - Training consistency
- Add time range filters: `--week`, `--month`, `--year`

**Task 7.2: AI Insights**
- Fetch recommendations from API
- Display training suggestions
- Show recovery status
- Highlight fatigue indicators
- Provide actionable advice

### Phase 8: Testing & Distribution (Week 10-11)
**Task 8.1: Testing**
- Unit tests for core logic (parsers, storage)
- Integration tests for API client
- Mock API server for testing
- Test offline mode thoroughly
- Test cross-platform compatibility (macOS, Linux, Windows)

**Task 8.2: Documentation**
- Write comprehensive README with examples
- Create user guide for common workflows
- Document configuration options
- Add troubleshooting section
- Include GIF demos of TUI

**Task 8.3: Release & Distribution**
- Set up GitHub releases with binaries
- Publish to crates.io: `cargo install ai-coach-cli`
- Create Homebrew formula for macOS
- Build Linux packages (deb/rpm)
- Create Docker image for containerized usage
- Set up auto-updates mechanism

**Task 8.4: CI/CD Pipeline**
- GitHub Actions for automated builds
- Cross-compilation for multiple platforms
- Automated tests on PRs
- Release automation with version bumps
- Generate checksums for binaries

## CLI Commands Reference

### Authentication
```bash
ai-coach login                  # Interactive login
ai-coach logout                 # Clear credentials
ai-coach whoami                 # Show current user
```

### Workout Management
```bash
ai-coach workout log            # Interactive workout logging
ai-coach workout log "Ran 5 miles"  # Natural language log
ai-coach workout list           # List recent workouts
ai-coach workout show <id>      # Show workout details
ai-coach workout edit <id>      # Edit workout
ai-coach workout delete <id>    # Delete workout
```

### Goals & Planning
```bash
ai-coach goals list             # List all goals
ai-coach goals create           # Create new goal
ai-coach goals update <id>      # Update goal
ai-coach goals complete <id>    # Mark goal complete
ai-coach plan show              # View training plan
```

### Analytics & Dashboard
```bash
ai-coach stats                  # Show training statistics
ai-coach stats --week           # Weekly stats
ai-coach dashboard              # Launch TUI dashboard
ai-coach insights               # Get AI recommendations
```

### Sync & Configuration
```bash
ai-coach sync                   # Sync with server
ai-coach sync --dry-run         # Preview sync changes
ai-coach config edit            # Edit configuration
ai-coach config show            # Show current config
```

### Utility
```bash
ai-coach version                # Show version info
ai-coach help                   # Show help
ai-coach completions bash       # Generate shell completions
```

## Configuration File Example

```toml
[api]
base_url = "https://api.ai-coach.app"
timeout_seconds = 30

[auth]
# Tokens are stored here after login
token = ""
refresh_token = ""

[sync]
auto_sync = true
conflict_resolution = "server_wins"  # Options: server_wins, local_wins, manual

[ui]
theme = "dark"                       # Options: dark, light
date_format = "%Y-%m-%d"
time_format = "24h"                  # Options: 24h, 12h
show_sync_status = true

[workouts]
default_distance_unit = "km"         # Options: km, miles
default_duration_unit = "minutes"
```

## TUI Dashboard Layout

```
╭─────────────────────────────────────────────────────╮
│ AI Coach - Dashboard                   [✓ Synced]   │
├─────────────────────────────────────────────────────┤
│                                                       │
│  This Week: 4 workouts | 32 km | 3h 45min           │
│  ████████████░░░░░░░░░░░░ 60% to weekly goal       │
│                                                       │
├──────────────────────┬──────────────────────────────┤
│ Recent Workouts      │ Upcoming Goals               │
│                      │                              │
│ • Sep 30: Running    │ • Half Marathon (15 days)    │
│   45min, 8.5km       │   Progress: 75% ████████░░   │
│                      │                              │
│ • Sep 29: Cycling    │ • Sub-20min 5K (45 days)     │
│   60min, 25km        │   Progress: 40% ████░░░░░░   │
│                      │                              │
│ • Sep 27: Running    │                              │
│   30min, 5km         │                              │
├──────────────────────┴──────────────────────────────┤
│ Quick Actions: [L]og workout  [G]oals  [S]tats      │
│ Navigation: ↑↓ scroll  [Q]uit  [?] Help             │
╰─────────────────────────────────────────────────────╯
```

## Success Metrics
- 100+ GitHub stars within 3 months
- 50+ installs via crates.io per week
- Positive feedback from developer community
- <5% crash rate
- Sub-second response for all commands

## Dependencies
- AI Coach REST API (existing)
- SQLite for local storage
- Internet connection for sync (optional)

## Risks & Mitigations
- **Risk**: Limited audience (only CLI users)
  - **Mitigation**: Position as community tool, open-source for contributions
- **Risk**: Offline data loss
  - **Mitigation**: Robust sync queue, automatic backups
- **Risk**: Cross-platform compatibility issues
  - **Mitigation**: Extensive testing on all platforms, use mature crates

## Future Enhancements
- Integration with tmux/zsh for inline stats
- Export to CSV/JSON
- Import from other platforms (Strava, Garmin)
- Plugin system for custom commands
- Multi-user support for teams
- Vim plugin integration
- Voice input for hands-free logging