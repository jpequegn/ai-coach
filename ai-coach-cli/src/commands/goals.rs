use anyhow::Result;

pub async fn list_goals(show_all: bool) -> Result<()> {
    println!("Goals");
    println!();
    println!("TODO: List goals (all: {})", show_all);

    Ok(())
}

pub async fn create_goal() -> Result<()> {
    println!("Create Goal");
    println!();
    println!("TODO: Interactive goal creation");

    Ok(())
}

pub async fn update_goal(id: &str) -> Result<()> {
    println!("Update Goal: {}", id);
    println!();
    println!("TODO: Update goal");

    Ok(())
}

pub async fn complete_goal(id: &str) -> Result<()> {
    println!("Complete Goal: {}", id);
    println!();
    println!("TODO: Mark goal as complete");

    Ok(())
}

pub async fn delete_goal(id: &str, force: bool) -> Result<()> {
    println!("Delete Goal: {}", id);
    println!();

    if !force {
        println!("TODO: Confirm deletion");
    }

    println!("TODO: Delete goal from storage");

    Ok(())
}

#[derive(clap::Args)]
pub struct GoalsCommand {}
