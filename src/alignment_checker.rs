#[path = "alignment_checker_check.rs"]
mod check;
#[path = "alignment_checker_cli.rs"]
pub mod cli;
#[path = "alignment_checker_index.rs"]
mod index;

pub use check::CheckSummary;

pub fn check_consistency(args: &cli::Args) -> anyhow::Result<CheckSummary> {
    let loaded_config = cli::LoadedConfig::from_path(&args.input)?;
    loaded_config.check_consistency()
}

pub fn plural(count: usize) -> &'static str {
    if count == 1 {
        ""
    } else {
        "s"
    }
}

pub fn entry_plural(count: usize) -> &'static str {
    if count == 1 {
        "y"
    } else {
        "ies"
    }
}
