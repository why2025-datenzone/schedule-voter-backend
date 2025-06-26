use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Optionally create a new user with their name, email, and password.
    #[arg(
        long, // The flag is --create-user
        num_args = 3, // It must be followed by exactly 3 values
        value_names = ["NAME", "EMAIL", "PASSWORD"], // Names for the help message
    )]
    create_user: Option<Vec<String>>,
}


fn main() {
	let cli = Cli::parse();

    // The `create_user` field will be Some(Vec<String>) if the user provided the flag,
    // and None otherwise.
    if let Some(user_data) = cli.create_user {
        // We know user_data has exactly 3 elements because of `num_args = 3`.
        // If the user provides a different number, clap will exit with an error.
        let name = &user_data[0];
        let email = &user_data[1];
        let password = &user_data[2]; // Be careful with passwords in real apps! See note below.
        api::main(Some((name.to_owned(), email.to_owned(), password.to_owned())))
    } else {
        api::main(None)
    }

}
