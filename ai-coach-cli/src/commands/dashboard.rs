use anyhow::Result;
use clap::Args;
use crate::ui::Dashboard;

#[derive(Args)]
pub struct DashboardCommand {}

impl DashboardCommand {
    pub async fn execute(self) -> Result<()> {
        let mut dashboard = Dashboard::new()?;
        dashboard.run()?;
        Ok(())
    }
}
