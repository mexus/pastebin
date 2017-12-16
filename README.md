# Simple pastebin server

## About

A simple multipurpose RESTful storage server written in
[Rust](https://www.rust-lang.org/). It uses [MongoDB](https://www.mongodb.com/)
as a storage backend and [Rocket](https://rocket.rs) web framework to do the web
stuff.

## Performance

I've run a simple performance test on my maching and the service is okay with
doing about 500 10 KB requests per second (in 5 threads). But we need better
performance tests.

## Limitations

As for now, there is a hard limit of **15 megabytes** on any incoming data. This
limitations comes from a [BSON document
size](https://docs.mongodb.com/manual/reference/limits/) with some reserve for
extra data.

While it is technically possible to store larger data chunks in a MongoDB using
a [GridFS](https://docs.mongodb.com/manual/core/gridfs/) it has not been
implemented in this project yet. But it is on the plan :)
