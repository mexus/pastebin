[![crates.io](https://img.shields.io/crates/v/pastebin.svg)](https://crates.io/crates/pastebin)
[![docs.rs](https://docs.rs/pastebin/badge.svg)](https://docs.rs/pastebin)
[![travis-ci](https://travis-ci.org/mexus/pastebin.svg?branch=master)](https://travis-ci.org/mexus/pastebin)

# Simple pastebin server

## About

A simple multipurpose RESTful storage server written in
[Rust](https://www.rust-lang.org/). It uses [MongoDB](https://www.mongodb.com/)
as a storage backend and [Iron](https://github.com/iron/iron) web framework to
do the web stuff.

## Performance

I've run a simple performance test on my maching and the service is able to
accept about 6500 post requests per second for 10 KB samples and about 3000
posts requests per seconds for 100 KB samples.

## Limitations

As for now, there is a hard limit of **15 megabytes** on any incoming data. This
limitations comes from a [BSON document
size](https://docs.mongodb.com/manual/reference/limits/) with some reserve for
extra data.

While it is technically possible to store larger data chunks in a MongoDB using
a [GridFS](https://docs.mongodb.com/manual/core/gridfs/) it has not been
implemented in this project yet. But it is on the plan :)
