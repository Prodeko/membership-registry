use clap::{Command, Arg, ArgAction};

pub fn build_cli() -> Command {
    Command::new("Membership registry")
        .author("Prodeko's Webbiteam")
        .about("Membership registry backend")
        .subcommand(Command::new("generate")
            .about("Generates sample data")
            .arg(Arg::new("amount")
                .short('n')
                .long("amount")
                .help("Amount of sample data to generate")
                .action(ArgAction::Set)
                .default_value("100")))
}
