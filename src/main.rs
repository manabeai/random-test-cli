use clap::{Parser, Subcommand};
use random_test_cli::{EDITOR_URL, browse, generate_sample_text};

#[derive(Debug, Parser)]
#[command(
    name = "rt",
    version,
    about = "Generate random tests from cp-ast share links",
    arg_required_else_help = true
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    /// cp-ast-ecosystems share URL or state value.
    link_or_state: Option<String>,

    /// Seed for deterministic sample generation.
    #[arg(long, global = true)]
    seed: Option<u64>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Open the cp-ast editor in the default browser.
    Open,
}

fn main() {
    if let Err(err) = run(Cli::parse()) {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    match cli.command {
        Some(Command::Open) => {
            browse::open_url(EDITOR_URL)?;
        }
        None => {
            let Some(input) = cli.link_or_state else {
                return Err("missing LINK_OR_STATE; try `rt --help`".into());
            };
            let (_seed, text) = generate_sample_text(&input, cli.seed)?;
            print!("{text}");
        }
    }
    Ok(())
}
