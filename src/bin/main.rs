use busy_bee::cli::Cli;
use clap::Parser;
use directories::ProjectDirs;

fn main() {
    let args = Cli::parse();

    let storage_dir = args.storage_dir.unwrap_or_else(|| {
        let default_dir = ProjectDirs::from("", "", "busy-bee")
            .map(|pd| pd.data_local_dir().to_path_buf());
        default_dir.expect(
            "Could not determine the local data directory for your OS. Please \
            use the '--storage-dir' flag to specify where any local data \
            should be saved.",
        )
    });

    match args.command {
        _ => todo!(),
    }
}
