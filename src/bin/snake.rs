use anyhow::Context;
use clap::Parser;

#[derive(Parser)]
struct Cli {
    #[arg(short, long)]
    fps: u32,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let args = game::Args::new(cli.fps).context("parsing command line arguments")?;

    game::render(args).await.context("rendering snake game")?;

    Ok(())
}
