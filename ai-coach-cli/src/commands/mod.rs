mod config_cmd;
mod dashboard;
mod goals;
mod login;
mod logout;
mod stats;
mod sync;
mod whoami;
mod workout;
mod workout_parser;

use anyhow::Result;
use clap::{Parser, Subcommand};

pub use dashboard::DashboardCommand;
pub use login::LoginCommand;
pub use logout::LogoutCommand;
pub use stats::StatsCommand;
pub use sync::SyncCommand;
pub use whoami::WhoamiCommand;
pub use workout::WorkoutCommand;

#[derive(Parser)]
#[command(name = "ai-coach")]
#[command(about = "Terminal-based training log application", long_about = None)]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Enable verbose logging
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Force offline mode (skip sync)
    #[arg(long, global = true)]
    offline: bool,

    /// Path to configuration file
    #[arg(long, global = true, env = "AI_COACH_CONFIG")]
    config: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Login to AI Coach
    Login(LoginCommand),

    /// Logout from AI Coach
    Logout(LogoutCommand),

    /// Show current user information
    Whoami(WhoamiCommand),

    /// Manage workouts
    #[command(subcommand)]
    Workout(WorkoutSubcommands),

    /// Manage goals
    #[command(subcommand)]
    Goals(GoalsSubcommands),

    /// Show training statistics
    Stats(StatsCommand),

    /// Sync data with server
    Sync(SyncCommand),

    /// Launch interactive dashboard
    Dashboard(DashboardCommand),

    /// Manage configuration
    #[command(subcommand)]
    Config(ConfigSubcommands),

    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
}

#[derive(Subcommand)]
enum WorkoutSubcommands {
    /// Log a new workout
    Log(WorkoutCommand),

    /// List recent workouts
    List {
        /// Filter by exercise type
        #[arg(short, long)]
        r#type: Option<String>,

        /// Filter from date (YYYY-MM-DD)
        #[arg(long)]
        from: Option<String>,

        /// Filter to date (YYYY-MM-DD)
        #[arg(long)]
        to: Option<String>,

        /// Number of workouts to show
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },

    /// Show workout details
    Show {
        /// Workout ID
        id: String,
    },

    /// Edit a workout
    Edit {
        /// Workout ID
        id: String,
    },

    /// Delete a workout
    Delete {
        /// Workout ID
        id: String,

        /// Skip confirmation prompt
        #[arg(short, long)]
        force: bool,
    },
}

#[derive(Subcommand)]
enum GoalsSubcommands {
    /// List all goals
    List {
        /// Show completed goals
        #[arg(short, long)]
        all: bool,
    },

    /// Create a new goal
    Create,

    /// Update a goal
    Update {
        /// Goal ID
        id: String,
    },

    /// Mark goal as complete
    Complete {
        /// Goal ID
        id: String,
    },

    /// Delete a goal
    Delete {
        /// Goal ID
        id: String,

        /// Skip confirmation prompt
        #[arg(short, long)]
        force: bool,
    },
}

#[derive(Subcommand)]
enum ConfigSubcommands {
    /// Show current configuration
    Show,

    /// Edit configuration file
    Edit,

    /// Initialize configuration with defaults
    Init {
        /// Overwrite existing config
        #[arg(short, long)]
        force: bool,
    },
}

impl Cli {
    pub async fn execute(self) -> Result<()> {
        // Set up logging level
        if self.verbose {
            tracing::info!("Verbose mode enabled");
        }

        match self.command {
            Commands::Login(cmd) => cmd.execute().await,
            Commands::Logout(cmd) => cmd.execute().await,
            Commands::Whoami(cmd) => cmd.execute().await,
            Commands::Workout(subcmd) => match subcmd {
                WorkoutSubcommands::Log(cmd) => cmd.execute().await,
                WorkoutSubcommands::List {
                    r#type,
                    from,
                    to,
                    limit,
                } => workout::list_workouts(r#type, from, to, limit).await,
                WorkoutSubcommands::Show { id } => workout::show_workout(&id).await,
                WorkoutSubcommands::Edit { id } => workout::edit_workout(&id).await,
                WorkoutSubcommands::Delete { id, force } => {
                    workout::delete_workout(&id, force).await
                }
            },
            Commands::Goals(subcmd) => match subcmd {
                GoalsSubcommands::List { all } => goals::list_goals(all).await,
                GoalsSubcommands::Create => goals::create_goal().await,
                GoalsSubcommands::Update { id } => goals::update_goal(&id).await,
                GoalsSubcommands::Complete { id } => goals::complete_goal(&id).await,
                GoalsSubcommands::Delete { id, force } => goals::delete_goal(&id, force).await,
            },
            Commands::Stats(cmd) => cmd.execute().await,
            Commands::Sync(cmd) => cmd.execute().await,
            Commands::Dashboard(cmd) => cmd.execute().await,
            Commands::Config(subcmd) => match subcmd {
                ConfigSubcommands::Show => config_cmd::show_config().await,
                ConfigSubcommands::Edit => config_cmd::edit_config().await,
                ConfigSubcommands::Init { force } => config_cmd::init_config(force).await,
            },
            Commands::Completions { shell } => {
                generate_completions(shell);
                Ok(())
            }
        }
    }
}

fn generate_completions(shell: clap_complete::Shell) {
    use clap::CommandFactory;
    use clap_complete::generate;
    use std::io;

    let mut cmd = Cli::command();
    let name = cmd.get_name().to_string();
    generate(shell, &mut cmd, name, &mut io::stdout());
}
