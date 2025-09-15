use clap::{Parser, Subcommand, ArgAction};
use rust_dv::helper::*;
use colored::Colorize;

#[derive(Parser, Debug)]
#[command(
    name = "cli",
    author = "Hao",
    version = "0.1.0",
    about = "A tool to manage the formal verification targets"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {

    #[command(
        name = "verify",
        about = "Verify the verification targets",
        alias = "v",
    )]
    Verify(VerifyArgs),

    #[command(
        name = "doc",
        about = "Generate documentation for the verification targets",
    )]
    Doc(DocArgs),

    #[command(
        name = "bootstrap",
        about = "Bootstrap the Verus toolchain",
        alias = "bs",
    )]
    Bootstrap(BootstrapArgs),

    #[command(
        name = "compile",
        about = "Compile the verification targets",
        alias = "c",
    )]
    Compile(CompileArgs),

    #[command(
        name = "fingerprint",
        about = "Print the fingerprint of the verification targets",
        alias = "fp",
    )]
    Fingerprint(FingerprintArgs),

    #[command(
        name = "list",
        about = "List all available verification targets",
        alias = "ls",
    )]
    ListTargets(ListTargetsArgs),

    #[command(
        name = "new",
        about = "Create a new verification target",
        alias = "n",
    )]
    NewTarget(NewTargetArgs),

    #[command(
        name = "show",
        about = "Show the details of a syntax item",
        alias = "s",
    )]
    ShowItem(ShowItemArgs),
}

#[derive(Parser, Debug)]
struct BootstrapArgs {
    #[arg(
        long = "vscode-extension",
        help = "Build verus vscode extension",
        default_value = "false", 
        action = ArgAction::SetTrue)]
    vscode_extension: bool,

    #[arg(
        long = "restart", 
        help = "Remove all toolchain and restart the bootstrap",
        default_value = "false", 
        action = ArgAction::SetTrue)]
    restart: bool,

    #[arg(
        long = "upgrade",
        help = "Upgrade the verus toolchain",
        default_value = "false",
        action = ArgAction::SetTrue)]
    upgrade: bool,

    #[arg(
        short = 'r',
        long = "debug",
        help = "Build artifacts in debug mode",
        default_value = "false",
        action = ArgAction::SetTrue
    )]
    debug: bool,

}

#[derive(Parser, Debug)]
struct VerifyArgs {
    #[arg(
        short = 't', 
        long = "targets", 
        value_parser = verus::find_target,
        help = "The targets to verify", 
        num_args = 0..,
        action = ArgAction::Append)]
    targets: Vec<VerusTarget>,

    #[arg(
        short = 'e', 
        long = "max-errors",
        help = "The maximum number of errors to display",
        default_value = "5", 
        action = ArgAction::Set)]
    max_errors: usize,

    #[arg(
        short = 'i', 
        long = "import", 
        value_parser = verus::find_target,
        help = "Import verified local crates (they need to be compiled first)",
        num_args = 0.., 
        action = ArgAction::Append)]
    imports: Vec<VerusTarget>,

    #[arg(
        short = 'l', 
        long = "log",
        help = "Log the verification process",
        default_value = "false", 
        action = ArgAction::SetTrue)]
    log: bool,

    #[arg(
        short = 'g',
        long = "trace",
        help = "Enable trace logging for the verification process",
        default_value = "false",
        action = ArgAction::SetTrue
    )]
    trace: bool,

    #[arg(
        short = 'd',
        long = "debug",
        help = "Build artifacts in debug mode",
        default_value = "false",
        action = ArgAction::SetTrue
    )]
    debug: bool,

    #[arg(
        last = true,
        help = "Pass-through arguments to the Verus verifier",
        allow_hyphen_values = true
    )]
    pass_through: Vec<String>,
}

#[derive(Parser, Debug)]
struct DocArgs {
    #[arg(
        short = 't', 
        long = "targets", 
        value_parser = verus::find_target,
        help = "The targets to generate document", 
        num_args = 0..,
        action = ArgAction::Append)]
    targets: Vec<VerusTarget>,
}

#[derive(Parser, Debug)]
struct CompileArgs {
    #[arg(
        short = 't', 
        long = "targets", 
        value_parser = verus::find_target,
        help = "The targets to compile", 
        num_args = 0..,
        action = ArgAction::Append)]
    targets: Vec<VerusTarget>,

    #[arg(
        short = 'i', 
        long = "import", 
        value_parser = verus::find_target,
        help = "Import verified local crates (they need to be compiled first)",
        num_args = 0.., 
        action = ArgAction::Append)]
    imports: Vec<VerusTarget>,

    #[arg(
        short = 'e', 
        long = "max-errors",
        help = "The maximum number of errors to display",
        default_value = "5", 
        action = ArgAction::Set)]
    max_errors: usize,

    #[arg(
        short = 'l', 
        long = "log",
        help = "Log the verification process",
        default_value = "false", 
        action = ArgAction::SetTrue)]
    log: bool,

    #[arg(
        short = 'g',
        long = "trace",
        help = "Enable trace logging for the verification process",
        default_value = "false",
        action = ArgAction::SetTrue
    )]
    trace: bool,

    #[arg(
        short = 'd', 
        long = "debug", 
        default_value = "false",
        help = "Build artifacts in debug mode",
        action = ArgAction::SetTrue)]
    debug: bool,

    #[arg(
        short = 'a', 
        long = "disasm", 
        default_value = "false",
        help = "Do not disassemble the compiled binary",
        action = ArgAction::SetTrue)]
    disasm: bool,

    #[arg(
        last = true,
        help = "Pass-through arguments to the Verus verifier",
        allow_hyphen_values = true
    )]
    pass_through: Vec<String>,
}

#[derive(Parser, Debug)]
struct FingerprintArgs {
    #[arg(
        short = 't', 
        long = "targets", 
        value_parser = verus::find_target,
        help = "The targets to fingerprint", 
        num_args = 0..,
        action = ArgAction::Append)]
    targets: Vec<VerusTarget>,
}

#[derive(Parser, Debug)]
struct ListTargetsArgs { }

#[derive(Parser, Debug)]
struct NewTargetArgs {
    #[arg(
        help = "Name of the new target (created under verification/)",
        allow_hyphen_values = true,
    )]
    name: String,
}

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum SupportedShowItem {
    Struct,
    Function,
}

#[derive(Parser, Debug)]
struct ShowItemArgs {
    /// Package name
    #[arg(
        short = 'p',
        long = "package",
        help = "The package to look into",
        action = ArgAction::Set,)]
    package: String,
    
    #[arg(
        short = 't',
        long = "info",
        help = "The type of information to show",
        value_enum,
        default_value = "struct",
    )]
    info_type: SupportedShowItem,

    #[arg(
        short = 'i',
        long = "id",
        help = "The identifier of the item to show",
        action = ArgAction::Set,
        required = true,
    )]
    id: String,
}

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum TranslateMode {
    Append,
    Overwrite,
}

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum TranslateBlockType {
    #[value(name = "func-context")]
    SolanaPerFunctionContext,
    Other,
}

#[derive(Parser, Debug)]
struct TranslateArgs {
    #[arg(
        short = 'p',
        long = "package",
        help = "The package to translate",
        action = ArgAction::Set,
        required = true,
    )]
    package: String,

    #[arg(
        short = 'i',
        long = "id",
        help = "The original identifier to translate",
        action = ArgAction::Set,
        required = true,
    )]
    id: String,

    #[arg(
        short = 'b',
        long = "block",
        help = "The type of block to translate (e.g., function context)",
        value_enum,
        default_value = "func-context",
    )]
    block_type: TranslateBlockType,
}

fn verify(args: &VerifyArgs) -> Result<(), DynError> {
    let targets = args.targets.clone();
    let imports = args.imports.clone();
    let options = verus::ExtraOptions {
        max_errors: args.max_errors,
        log: args.log,
        release: !args.debug,
        trace: args.trace,
        disasm: false,
        pass_through: args.pass_through.clone(),
    };

    verus::exec_verify(&targets, &imports, &options)
}

fn doc(_args: &DocArgs) -> Result<(), DynError> {
    error!("The doc command is not implemented yet");
}

fn bootstrap(args: &BootstrapArgs) -> Result<(), DynError> {
    let options = verus::install::VerusInstallOpts {
        vscode_extension: args.vscode_extension,
        release: !args.debug,
        restart: args.restart,
    };
    
    if args.upgrade {
        verus::install::exec_upgrade(&options)
    } else {
        verus::install::exec_bootstrap(&options)
    }
}

fn compile(args: &CompileArgs) -> Result<(), DynError> {
    let targets = args.targets.clone();
    let imports = args.imports.clone();
    let options = verus::ExtraOptions {
        max_errors: args.max_errors,
        log: args.log,
        trace: args.trace,
        release: !args.debug,
        disasm: args.disasm,
        pass_through: args.pass_through.clone(),
    };

    verus::exec_compile(&targets, &imports, &options)
}

fn fingerprint(args: &FingerprintArgs) -> Result<(), DynError> {
    let targets = args.targets.clone();
    for target in targets {
        println!("{}: {}", target.name, target.fingerprint());
    }
    Ok(())
}

fn list_targets(_args: &ListTargetsArgs) -> Result<(), DynError> {
    let targets = verus::verus_targets();
    let width = targets.keys()
        .map(|s| s.len())
        .max()
        .unwrap_or(0)
        .min(70) + 1;

    for (name, target) in targets {
        println!("{:<width$}: {} {}\n  {}",
            name.blue(),
            target.dir.to_string_lossy().bright_black(),
            target.version.bright_yellow(),
            format!("{:<width$}features = [{}]",
                "",
                target.features.join(", "))
                .bright_black(),
        );
    }
    Ok(())
}

fn new_target(args: &NewTargetArgs) -> Result<(), DynError> {
    if args.name.is_empty() || args.name.trim().is_empty() {
        error!("Please provide a name for the new target.");
    }

    let package = crate::new::create(args.name.trim());
    println!("Created new target: {}", package.name);
    Ok(())
}

fn show_item(args: &ShowItemArgs) -> Result<(), DynError> {
    let package = &args.package;
    let id = &args.id;

    match args.info_type {
        SupportedShowItem::Struct => {
            let struct_info = show::find_struct_in_package(package, id)?;
            struct_info.iter()
                .for_each(|s| println!("{}", s.as_string()));
        },
        _ => {
            error!("Unsupported item type: {:?}", args.info_type);
        }
    }

    Ok(())
}


fn main() {
    let cli = Cli::parse();
    if let Err(e) = match &cli.command {
        Commands::Verify(args) => { verify(args) }
        Commands::Doc(args) => { doc(args) }
        Commands::Bootstrap(args) => { bootstrap(args) }
        Commands::Compile(args) => { compile(args) }
        Commands::Fingerprint(args) => { fingerprint(args) }
        Commands::ListTargets(args) => { list_targets(args) }
        Commands::NewTarget(args) => { new_target(args) }
        Commands::ShowItem(args) => { show_item(args) }
    } {
        error!("Error when executing command `{:?}`: {}", cli.command, e);
    }
}
