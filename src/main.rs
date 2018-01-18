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
extern crate tera;

mod cmdargs;
mod mongo_impl;

use iron::error::HttpError;
use mongo_driver::MongoError;
use mongo_driver::client::ClientPool;
use mongo_impl::MongoDbWrapper;
use tera::Tera;

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
        HttpError(err: HttpError) {
            cause(err)
            from()
        }
        Tera(err: tera::Error) {
            cause(err)
            from()
        }
    }
}

fn init_logs(verbose: usize) -> Result<(), Error> {
    // Set up the logging depending on how many times a '-v' option has been used.
    let verbosity = match verbose {
        1 => simplelog::LogLevelFilter::Warn,
        2 => simplelog::LogLevelFilter::Info,
        3 => simplelog::LogLevelFilter::Debug,
        4 => simplelog::LogLevelFilter::Trace,
        _ => simplelog::LogLevelFilter::Error,
    };
    simplelog::SimpleLogger::init(verbosity, Default::default()).unwrap();
    Ok(())
}

fn run() -> Result<(), Error> {
    let options = cmdargs::parse()?;
    init_logs(options.verbose)?;
    let mongo_client_pool = ClientPool::new(options.db_options.uri.clone(), None);
    let db_wrapper = MongoDbWrapper::new(options.db_options.db_name,
                                         options.db_options.collection_name,
                                         mongo_client_pool);
    let templates =
        Tera::new(&format!("{}/**/*{}", options.templates_path, options.templates_ext))?;
    pastebin::web::run_web(db_wrapper, options.web_addr, templates, &options.url_prefix)?;
    unreachable!()
}

fn main() {
    match run() {
        Ok(_) => {}
        Err(e) => error!["Caught an error: {:?}", e],
    }
}
