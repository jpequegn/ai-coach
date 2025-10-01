use anyhow::{Context, Result};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    Frame, Terminal,
};
use std::io;

use super::app::{App, Panel};
use super::widgets;

/// Dashboard manages the TUI lifecycle
pub struct Dashboard {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    app: App,
}

impl Dashboard {
    /// Create new dashboard instance
    pub fn new() -> Result<Self> {
        // Setup terminal
        enable_raw_mode().context("Failed to enable raw mode")?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)
            .context("Failed to setup terminal")?;

        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend).context("Failed to create terminal")?;

        // Create app state
        let app = App::new().context("Failed to initialize app")?;

        Ok(Self { terminal, app })
    }

    /// Run the dashboard event loop
    pub fn run(&mut self) -> Result<()> {
        loop {
            let app = &self.app;
            self.terminal.draw(|f| ui(f, app))?;

            if event::poll(std::time::Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == event::KeyEventKind::Press {
                        self.app.handle_key(key.code)?;
                    }
                }
            }

            if self.app.should_quit {
                break;
            }
        }

        Ok(())
    }

    /// Cleanup terminal on exit
    pub fn cleanup(&mut self) -> Result<()> {
        disable_raw_mode().context("Failed to disable raw mode")?;
        execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )
        .context("Failed to restore terminal")?;
        self.terminal.show_cursor().context("Failed to show cursor")?;

        Ok(())
    }
}

impl Drop for Dashboard {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}

/// Render the UI
fn ui(f: &mut Frame, app: &App) {
        let size = f.area();

        // Main layout: top area + status bar
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .split(size);

        // Split main area into columns
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(main_chunks[0]);

        // Left column: Weekly summary (top) + Recent workouts (bottom)
        let left_panels = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(columns[0]);

        // Split weekly summary area: stats (top) + chart (bottom)
        let summary_panels = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(8), Constraint::Min(0)])
            .split(left_panels[0]);

        // Right column: Goals (top) + Quick actions (bottom)
        let right_panels = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(columns[1]);

        // Render widgets
        widgets::render_weekly_summary(
            summary_panels[0],
            f.buffer_mut(),
            &app.weekly_summary,
            app.selected_panel == Panel::WeeklySummary,
        );

        widgets::render_weekly_chart(
            summary_panels[1],
            f.buffer_mut(),
            &app.weekly_summary,
        );

        widgets::render_recent_workouts(
            left_panels[1],
            f.buffer_mut(),
            &app.recent_workouts,
            app.selected_index,
            app.selected_panel == Panel::RecentWorkouts,
        );

        widgets::render_goals(
            right_panels[0],
            f.buffer_mut(),
            app.selected_panel == Panel::Goals,
        );

        widgets::render_quick_actions(
            right_panels[1],
            f.buffer_mut(),
            app.selected_index,
            app.selected_panel == Panel::QuickActions,
            app.sync_pending,
        );

        // Render status bar
        widgets::render_status_bar(main_chunks[1], f.buffer_mut(), app.sync_pending);

        // Render help overlay if active
        if app.show_help {
            let help_area = centered_rect(60, 80, size);
            widgets::render_help_overlay(help_area, f.buffer_mut());
        }
}

/// Helper function to create a centered rect
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
