# AI Coach CLI - User Guide

Complete guide to using AI Coach CLI effectively for training management and goal tracking.

## Table of Contents

1. [Getting Started](#getting-started)
2. [Workout Logging](#workout-logging)
3. [Goal Management](#goal-management)
4. [TUI Dashboard](#tui-dashboard)
5. [Syncing and Offline Mode](#syncing-and-offline-mode)
6. [Advanced Features](#advanced-features)
7. [Tips and Best Practices](#tips-and-best-practices)

## Getting Started

### First-Time Setup

After installing AI Coach CLI, initialize your configuration:

```bash
# Create configuration file
ai-coach config init

# Login to your account
ai-coach login
```

You'll be prompted for your credentials. The CLI will store your authentication token securely in `~/.ai-coach/config.toml`.

### Verifying Installation

```bash
# Check version
ai-coach --version

# View help
ai-coach --help

# Check current user
ai-coach whoami
```

## Workout Logging

### Interactive Logging

The simplest way to log a workout is interactively:

```bash
ai-coach workout log
```

You'll be prompted for:
1. Exercise type (running, cycling, swimming, strength, walking)
2. Duration
3. Distance (if applicable)
4. Notes

### Natural Language Logging

AI Coach CLI understands natural language descriptions:

#### Running Workouts

```bash
# Simple runs
ai-coach workout log "Ran 5 miles"
ai-coach workout log "Morning 10k run"

# With pace/time
ai-coach workout log "Ran 5 miles in 40 minutes"
ai-coach workout log "10k in 52:30"

# With description
ai-coach workout log "Easy recovery run, 5 miles in 45 min"
ai-coach workout log "Interval training: 8x400m repeats"
```

**Supported formats:**
- `"Ran X miles"` or `"Ran X km"`
- `"X mile run"` or `"X km run"`
- `"Ran X miles in Y minutes"`
- `"Running for X minutes"`

#### Cycling Workouts

```bash
# Distance-based
ai-coach workout log "Bike ride 25km"
ai-coach workout log "Cycling 15 miles"

# Time-based
ai-coach workout log "60 min bike ride"
ai-coach workout log "Evening cycling session, 90 minutes"

# With both
ai-coach workout log "Bike ride 25km in 60 minutes"
ai-coach workout log "Cycled 30 miles in 2 hours"
```

**Supported formats:**
- `"Bike ride X km"` or `"Bike ride X miles"`
- `"Cycling X km"` or `"Cycling X miles"`
- `"X min bike ride"`
- `"Biked X km in Y minutes"`

#### Swimming Workouts

```bash
# Distance-based
ai-coach workout log "Swam 2000 meters"
ai-coach workout log "Pool workout, 1500m"

# Time-based
ai-coach workout log "Swimming for 45 minutes"
ai-coach workout log "30 min swim session"`

# With both
ai-coach workout log "Swam 2000 meters in 40 minutes"
ai-coach workout log "1 mile swim in 35 min"
```

**Supported formats:**
- `"Swam X meters"` or `"Swam X yards"`
- `"Swimming for X minutes"`
- `"X meter swim"`
- `"Swam X meters in Y minutes"`

#### Strength Training

```bash
# Basic
ai-coach workout log "Strength training for 60 minutes"
ai-coach workout log "Weight lifting 45 min"

# With focus
ai-coach workout log "Upper body workout, 50 minutes"
ai-coach workout log "Leg day, 60 min"
ai-coach workout log "Full body strength, 75 minutes"
```

**Supported formats:**
- `"Strength training for X minutes"`
- `"Weight lifting X min"`
- `"Gym session X minutes"`
- `"Upper/lower/full body workout"`

#### Walking

```bash
# Simple
ai-coach workout log "Walked 3 miles"
ai-coach workout log "Morning walk 5km"

# With time
ai-coach workout log "Walked 3 miles in 50 minutes"
ai-coach workout log "45 min walk"
```

### Command-Line Flags

Use flags for more control:

```bash
# Specify workout type
ai-coach workout log --type running --distance 5 --duration 40

# Add notes
ai-coach workout log --type cycling --distance 25 --duration 60 --notes "Hilly route"

# Quick log with defaults
ai-coach workout log -t running -d 10 --time 60
```

### Viewing Workouts

```bash
# List recent workouts
ai-coach workout list

# List with limit
ai-coach workout list --limit 20

# Filter by type
ai-coach workout list --type running

# Filter by date range
ai-coach workout list --from 2024-01-01 --to 2024-01-31
```

### Editing and Deleting

```bash
# Show workout details
ai-coach workout show <workout-id>

# Edit a workout
ai-coach workout edit <workout-id>

# Delete a workout
ai-coach workout delete <workout-id>

# Delete without confirmation
ai-coach workout delete <workout-id> --force
```

## Goal Management

### Creating Goals

```bash
# Interactive goal creation
ai-coach goals create
```

You'll be prompted for:
- Goal type (distance, time, frequency, weight)
- Target value
- Target date
- Description

### Examples

```bash
# Create goals for different objectives
ai-coach goals create
# "Run 100 miles this month"
# "Complete 5k in under 25 minutes"
# "Train 5 days per week"
```

### Tracking Progress

```bash
# View all goals
ai-coach goals list

# Show specific goal with progress
ai-coach goals show <goal-id>

# Check weekly progress
ai-coach stats --period week
```

### Managing Goals

```bash
# Update a goal
ai-coach goals update <goal-id>

# Mark as complete
ai-coach goals complete <goal-id>

# Delete a goal
ai-coach goals delete <goal-id>
```

## TUI Dashboard

Launch the interactive terminal dashboard:

```bash
ai-coach dashboard
```

### Dashboard Panels

The dashboard includes:

1. **Overview Panel**: Current week summary
2. **Workout History**: Recent training sessions
3. **Goals**: Active goals with progress bars
4. **Statistics**: Training metrics and trends
5. **Calendar**: Training calendar view

### Navigation

| Key | Action |
|-----|--------|
| `↑`/`↓` | Navigate within panel |
| `←`/`→` | Switch between panels |
| `Tab` | Next panel |
| `Shift+Tab` | Previous panel |
| `Enter` | Select/view details |
| `r` | Refresh data |
| `s` | Sync with server |
| `n` | New workout |
| `g` | Toggle goals view |
| `h` | Show help |
| `q`/`Ctrl+C` | Quit |

### Dashboard Tips

- The dashboard auto-refreshes every 30 seconds
- Use `s` to manually sync with the server
- Press `n` to quick-log a workout without leaving the dashboard
- Press `/` to search through workouts

## Syncing and Offline Mode

### Manual Sync

```bash
# Sync with server
ai-coach sync

# Force sync (resolve conflicts automatically)
ai-coach sync --force
```

### Auto-Sync

Enable auto-sync in configuration:

```toml
[sync]
auto_sync = true
sync_interval = 300  # seconds
```

With auto-sync enabled, the CLI syncs after each command.

### Offline Mode

AI Coach CLI works fully offline:

1. **Log workouts offline**: Data stored locally in sled database
2. **View history**: Access all local data
3. **Edit goals**: Modify local goals
4. **Queue for sync**: Changes marked for upload

When you come back online:

```bash
# Sync all offline changes
ai-coach sync
```

### Conflict Resolution

Configure how to handle sync conflicts:

```toml
[sync]
conflict_resolution = "server_wins"  # or "local_wins" or "manual"
```

- **server_wins**: Server data overwrites local
- **local_wins**: Local data overwrites server
- **manual**: Prompt for each conflict

## Advanced Features

### Configuration Management

```bash
# Show current configuration
ai-coach config show

# Edit configuration file
ai-coach config edit

# Reset to defaults
ai-coach config init --reset
```

### Environment Variables

Override config with environment variables:

```bash
# Set API URL
export AI_COACH_API_URL="https://api.example.com"

# Set theme
export AI_COACH_THEME="light"

# Enable debug logging
export RUST_LOG=debug

# Run with overrides
ai-coach workout list
```

### Shell Completions

Generate completions for faster command entry:

```bash
# Bash
ai-coach completions bash > ~/.bash_completions/ai-coach
source ~/.bash_completions/ai-coach

# Zsh
ai-coach completions zsh > ~/.zfunc/_ai-coach

# Fish
ai-coach completions fish > ~/.config/fish/completions/ai-coach.fish
```

Then use Tab completion:

```bash
ai-coach work<Tab>
# Expands to: ai-coach workout

ai-coach workout l<Tab>
# Shows: list, log
```

### Batch Operations

Use shell scripting for batch operations:

```bash
# Log multiple workouts from file
while IFS= read -r workout; do
  ai-coach workout log "$workout"
done < workouts.txt

# Export workouts to CSV
ai-coach workout list --format json | jq -r '.[] | [.date, .type, .distance, .duration] | @csv' > workouts.csv
```

## Tips and Best Practices

### Daily Workflow

```bash
# Morning: Log yesterday's workout
ai-coach workout log "Ran 5 miles in 42 minutes"

# Check progress
ai-coach stats --period week

# Quick dashboard view
ai-coach dashboard
```

### Weekly Review

```bash
# View weekly stats
ai-coach stats --period week

# Check goal progress
ai-coach goals list

# Review workouts
ai-coach workout list --limit 7
```

### Natural Language Tips

For best results with natural language:

1. **Be specific about units**: Use "miles" or "km", "meters" or "yards"
2. **Include time when possible**: "in X minutes" for better accuracy
3. **Use common terms**: "ran", "bike ride", "swam", etc.
4. **Add context in notes**: Use `--notes` for details not in description

### Performance Tips

1. **Use offline mode**: Log workouts offline, sync in batches
2. **Limit history**: Use `--limit` when listing workouts
3. **Filter by type**: Narrow searches with `--type`
4. **Use dashboard**: More efficient than multiple CLI calls

### Troubleshooting

#### Sync Issues

```bash
# Check sync status
ai-coach config show | grep sync

# View unsynced items
ai-coach workout list --filter unsynced

# Force full sync
ai-coach sync --force --full
```

#### Authentication Issues

```bash
# Refresh session
ai-coach logout
ai-coach login

# Check token status
ai-coach whoami
```

#### Database Issues

```bash
# Backup database
cp -r ~/.ai-coach/db ~/.ai-coach/db.backup

# Reset database (WARNING: deletes local data)
rm -rf ~/.ai-coach/db
ai-coach sync
```

## Common Workflows

### Marathon Training

```bash
# Week 1
ai-coach workout log "Long run 10 miles in 90 min"
ai-coach workout log "Easy run 5 miles"
ai-coach workout log "Interval training 6 miles"

# Check weekly mileage
ai-coach stats --period week
```

### Strength Program

```bash
# Day 1: Upper body
ai-coach workout log "Upper body strength 60 min"

# Day 2: Lower body
ai-coach workout log "Lower body strength 60 min"

# Day 3: Full body
ai-coach workout log "Full body workout 75 min"

# Track frequency goal
ai-coach goals list
```

### Cross-Training

```bash
# Mixed week
ai-coach workout log "Ran 5 miles"
ai-coach workout log "Bike ride 20km"
ai-coach workout log "Swimming 1500m"
ai-coach workout log "Strength training 45 min"

# View variety
ai-coach workout list --group-by type
```

## Next Steps

- Explore the [README](README.md) for installation options
- Check [troubleshooting](README.md#troubleshooting) for common issues
- Report bugs at [GitHub Issues](https://github.com/jpequegn/ai-coach/issues)
- Join discussions at [GitHub Discussions](https://github.com/jpequegn/ai-coach/discussions)

---

**Happy Training! 🏃💪**
