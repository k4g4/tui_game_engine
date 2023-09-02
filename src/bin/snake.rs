use anyhow::Context;

fn main() -> anyhow::Result<()> {
    game::render().context("failed during render")?;

    Ok(())
}
