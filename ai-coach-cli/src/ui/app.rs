use crate::models::Workout;
use crate::storage::Storage;
use anyhow::Result;
use chrono::{DateTime, Datelike, Utc};

/// Application state for the TUI dashboard
pub struct App {
    /// Should the application quit?
    pub should_quit: bool,
    /// Currently selected panel
    pub selected_panel: Panel,
    /// Selected index in the current panel
    pub selected_index: usize,
    /// Show help overlay
    pub show_help: bool,
    /// Recent workouts
    pub recent_workouts: Vec<Workout>,
    /// Weekly summary data
    pub weekly_summary: WeeklySummary,
    /// Sync status
    pub sync_pending: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Panel {
    WeeklySummary,
    RecentWorkouts,
    Goals,
    QuickActions,
}

#[derive(Debug, Clone)]
pub struct WeeklySummary {
    pub total_workouts: usize,
    pub total_distance_km: f64,
    pub total_duration_min: u32,
    pub workouts_by_day: Vec<usize>, // 7 days, Monday to Sunday
    pub distance_by_day: Vec<f64>,   // 7 days
}

impl Default for WeeklySummary {
    fn default() -> Self {
        Self {
            total_workouts: 0,
            total_distance_km: 0.0,
            total_duration_min: 0,
            workouts_by_day: vec![0; 7],
            distance_by_day: vec![0.0; 7],
        }
    }
}

impl App {
    /// Create new app instance and load data
    pub fn new() -> Result<Self> {
        let storage = Storage::init()?;

        // Load recent workouts (last 10)
        let all_workouts = storage.list_workouts()?;
        let recent_workouts: Vec<Workout> = all_workouts.into_iter().take(10).collect();

        // Calculate weekly summary
        let weekly_summary = Self::calculate_weekly_summary(&storage)?;

        // Get sync queue size
        let sync_pending = storage.get_unsynced_workouts()?.len();

        Ok(Self {
            should_quit: false,
            selected_panel: Panel::WeeklySummary,
            selected_index: 0,
            show_help: false,
            recent_workouts,
            weekly_summary,
            sync_pending,
        })
    }

    /// Calculate weekly summary from stored workouts
    fn calculate_weekly_summary(storage: &Storage) -> Result<WeeklySummary> {
        let now = Utc::now();
        let week_start = now - chrono::Duration::days(7);

        let workouts = storage.list_workouts()?;

        let mut summary = WeeklySummary::default();

        for workout in workouts {
            if workout.date >= week_start {
                summary.total_workouts += 1;

                if let Some(distance) = workout.distance_km {
                    summary.total_distance_km += distance;
                }

                if let Some(duration) = workout.duration_minutes {
                    summary.total_duration_min += duration;
                }

                // Calculate day of week (0 = Monday, 6 = Sunday)
                let day_of_week = workout.date.weekday().num_days_from_monday() as usize;
                summary.workouts_by_day[day_of_week] += 1;

                if let Some(distance) = workout.distance_km {
                    summary.distance_by_day[day_of_week] += distance;
                }
            }
        }

        Ok(summary)
    }

    /// Refresh data from storage
    pub fn refresh(&mut self) -> Result<()> {
        let storage = Storage::init()?;

        let all_workouts = storage.list_workouts()?;
        self.recent_workouts = all_workouts.into_iter().take(10).collect();

        self.weekly_summary = Self::calculate_weekly_summary(&storage)?;
        self.sync_pending = storage.get_unsynced_workouts()?.len();

        Ok(())
    }

    /// Handle keyboard input
    pub fn handle_key(&mut self, key: crossterm::event::KeyCode) -> Result<()> {
        use crossterm::event::KeyCode;

        // Help overlay takes precedence
        if self.show_help {
            match key {
                KeyCode::Char('?') | KeyCode::Esc => self.show_help = false,
                _ => {}
            }
            return Ok(());
        }

        match key {
            // Quit
            KeyCode::Char('q') | KeyCode::Char('Q') => {
                self.should_quit = true;
            }

            // Help
            KeyCode::Char('?') => {
                self.show_help = true;
            }

            // Quick actions
            KeyCode::Char('l') | KeyCode::Char('L') => {
                // TODO: Open workout log dialog
                // For now, just refresh
                self.refresh()?;
            }

            KeyCode::Char('g') | KeyCode::Char('G') => {
                // TODO: Open goals view
            }

            KeyCode::Char('s') | KeyCode::Char('S') => {
                // TODO: Open stats view
            }

            KeyCode::Char('y') | KeyCode::Char('Y') => {
                // TODO: Trigger sync
            }

            KeyCode::Char('r') | KeyCode::Char('R') => {
                self.refresh()?;
            }

            // Tab to switch panels
            KeyCode::Tab => {
                self.next_panel();
            }

            KeyCode::BackTab => {
                self.prev_panel();
            }

            // Navigation within panel
            KeyCode::Up | KeyCode::Char('k') => {
                self.move_selection_up();
            }

            KeyCode::Down | KeyCode::Char('j') => {
                self.move_selection_down();
            }

            KeyCode::Left | KeyCode::Char('h') => {
                self.prev_panel();
            }

            // Note: 'l' is used for "Log workout", so only arrow key for right
            KeyCode::Right => {
                self.next_panel();
            }

            _ => {}
        }

        Ok(())
    }

    /// Move to next panel
    fn next_panel(&mut self) {
        self.selected_panel = match self.selected_panel {
            Panel::WeeklySummary => Panel::RecentWorkouts,
            Panel::RecentWorkouts => Panel::Goals,
            Panel::Goals => Panel::QuickActions,
            Panel::QuickActions => Panel::WeeklySummary,
        };
        self.selected_index = 0;
    }

    /// Move to previous panel
    fn prev_panel(&mut self) {
        self.selected_panel = match self.selected_panel {
            Panel::WeeklySummary => Panel::QuickActions,
            Panel::RecentWorkouts => Panel::WeeklySummary,
            Panel::Goals => Panel::RecentWorkouts,
            Panel::QuickActions => Panel::Goals,
        };
        self.selected_index = 0;
    }

    /// Move selection up within current panel
    fn move_selection_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    /// Move selection down within current panel
    fn move_selection_down(&mut self) {
        let max_index = match self.selected_panel {
            Panel::RecentWorkouts => self.recent_workouts.len().saturating_sub(1),
            Panel::QuickActions => 3, // 4 quick actions (0-3)
            _ => 0,
        };

        if self.selected_index < max_index {
            self.selected_index += 1;
        }
    }
}
