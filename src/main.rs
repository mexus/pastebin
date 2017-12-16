extern crate env_logger;
#[macro_use]
extern crate log;
extern crate pastebin;
#[macro_use]
extern crate quick_error;

use pastebin::{web, DbOptions, MongoError, RocketError};
use pastebin::mongo_impl::MongoDbWrapper;

mod cmdargs;

quick_error! {
    #[derive(Debug)]
    pub enum Error {
        Args(err: cmdargs::Error) {
            cause(err)
            from()
        }
        Mongo(err: MongoError) {
            cause(err)
            from()
        }
        Rocket(err: RocketError) {
            cause(err)
            from()
        }
        Logger(err: log::SetLoggerError) {
            cause(err)
            from()
        }
    }
}

fn init_logs() -> Result<(), Error> {
    let mut builder = env_logger::LogBuilder::new();
    builder.filter(None, log::LogLevelFilter::Info)
           .filter(Some("pastebin::mongo_impl"), log::LogLevelFilter::Trace)
           .init()?;
    Ok(())
}

fn run() -> Result<(), Error> {
    init_logs()?;
    let config = cmdargs::parse()?;
    let db_wrapper = MongoDbWrapper::new(config);
    let error = web::run_web(Box::new(db_wrapper));
    Err(error.into())
}

fn main() {
    match run() {
        Ok(_) => {}
        Err(e) => error!["Caught an error: {:?}", e],
    }
}
