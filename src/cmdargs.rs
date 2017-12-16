extern crate clap;

use DbOptions;
use std::num;

quick_error! {
    /// Commandline parsing errors.
    #[derive(Debug)]
    pub enum Error {
        /// A required argument has not been provided.
        NoArgument(arg: String) {
            description("Argument not found")
            display("No argument '{}' provided", arg)
        }
        /// Can't parse a value of an argument.
        ParseInt(err: num::ParseIntError) {
            cause(err)
            from()
        }
    }
}

/// A helper to simplify a creation of a "no argument" error.
fn no_arg(arg: &str) -> Error {
    Error::NoArgument(arg.into())
}

/// Parses command line arguments.
pub fn parse() -> Result<DbOptions, Error> {
    let args = build_cli().get_matches();
    let host = args.value_of("DB_HOST").ok_or(no_arg("DB_HOST"))?
                   .to_string();
    let port: u16 = args.value_of("DB_PORT").ok_or(no_arg("DB_PORT"))?.parse()?;
    let db_name = args.value_of("DB_NAME").ok_or(no_arg("DB_NAME"))?
                      .to_string();
    let db_user = args.value_of("DB_USER").map(str::to_string);
    let db_pass = args.value_of("DB_PASS").map(str::to_string);
    let collection_name = args.value_of("COLLECTION_NAME").ok_or(no_arg("COLLECTION_NAME"))?
                              .to_string();
    Ok(DbOptions { host,
                   port,
                   db_name,
                   db_user,
                   db_pass,
                   collection_name, })
}

/// Builds command line arguments.
fn build_cli() -> clap::App<'static, 'static> {
    use self::clap::{App, Arg};
    App::new("Pastebin web server").about("Launches a pastebin web server.")
                                   .arg(Arg::with_name("DB_HOST").long("db-host")
                                                                 .value_name("host")
                                                                 .takes_value(true)
                                                                 .default_value("localhost")
                                                                 .help("Database host"))
                                   .arg(Arg::with_name("DB_PORT").long("db-port")
                                                                 .value_name("port")
                                                                 .takes_value(true)
                                                                 .default_value("27017")
                                                                 .help("Database port"))
                                   .arg(Arg::with_name("DB_NAME").long("db-name")
                                                                 .value_name("name")
                                                                 .takes_value(true)
                                                                 .required(true)
                                                                 .help("Name of the database"))
                                   .arg(Arg::with_name("DB_USER").long("user")
                                                                 .value_name("name")
                                                                 .takes_value(true)
                                                                 .help("User name"))
                                   .arg(Arg::with_name("DB_PASS").long("password")
                                                                 .value_name("password")
                                                                 .takes_value(true)
                                                                 .help("Password"))
                                   .arg(Arg::with_name("COLLECTION_NAME").long("collection")
                                                                         .value_name("name")
                                                                         .takes_value(true)
                                                                         .required(true)
                                                                         .help("Collection name"))
}
