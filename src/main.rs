#[macro_use]
extern crate log;
extern crate pastebin;
#[macro_use]
extern crate quick_error;
extern crate simplelog;

use pastebin::{web, DbOptions, HttpError, MongoError};
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
    let db_wrapper = MongoDbWrapper::new(options.db_options);
    let mut _web = web::run_web(Box::new(db_wrapper))?;
    Ok(())
}

fn main() {
    match run() {
        Ok(_) => {}
        Err(e) => error!["Caught an error: {:?}", e],
    }
}
