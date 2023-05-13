use std::path::PathBuf;

#[derive(clap::Parser)]
struct LitWebCli {
    #[command(subcommand)]
    subcommand: LitWebSubcmd,
}

#[derive(clap::Subcommand)]
enum LitWebSubcmd {
    Tangle(LitWebTangleArgs),
}

#[derive(clap::Parser)]
struct LitWebTangleArgs {
    input: PathBuf,
    #[arg(short = 'O')]
    output: Option<PathBuf>,
    #[arg(long, short)]
    force: bool,
}

fn main() {
    use clap::Parser;
    let cli = LitWebCli::parse();
    match cli.subcommand {
        LitWebSubcmd::Tangle(tangle_args) => {
            if let Err(err) = litweb::tangle_or_untangle(
                &tangle_args.input,
                tangle_args.output.as_deref(),
                tangle_args.force,
            ) {
                eprintln!("ERROR: {err}");
            }
        }
    }
}
