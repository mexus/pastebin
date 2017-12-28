#[macro_use]
extern crate bson;
extern crate iron;
#[macro_use]
extern crate log;
extern crate mongo_driver;
extern crate pastebin;
#[macro_use]
extern crate quick_error;
extern crate simplelog;

mod cmdargs;
mod mongo_impl;

use iron::error::HttpError;
use mongo_driver::MongoError;
use mongo_driver::client::ClientPool;
use mongo_impl::MongoDbWrapper;

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
        Logger(err: simplelog::TermLogError) {
            cause(err)
            from()
        }
        HttpError(err: HttpError) {
            cause(err)
            from()
        }
    }
}

fn init_logs(verbose: usize) -> Result<(), Error> {
    // Set up the logging depending on how many times a '-v' option has been used.
    simplelog::TermLogger::init(match verbose {
                                    1 => log::LogLevelFilter::Warn,
                                    2 => log::LogLevelFilter::Info,
                                    3 => log::LogLevelFilter::Debug,
                                    4 => log::LogLevelFilter::Trace,
                                    _ => log::LogLevelFilter::Error,
                                },
                                Default::default())?;
    Ok(())
}

fn run() -> Result<(), Error> {
    let options = cmdargs::parse()?;
    init_logs(options.verbose)?;
    let mongo_client_pool = ClientPool::new(options.db_options.uri.clone(), None);
    let db_wrapper = MongoDbWrapper::new(options.db_options.db_name,
                                         options.db_options.collection_name,
                                         mongo_client_pool);
    pastebin::web::run_web(db_wrapper, options.web_addr)?;
    unreachable!()
}

fn main() {
    match run() {
        Ok(_) => {}
        Err(e) => error!["Caught an error: {:?}", e],
    }
}
