[![crates.io](https://img.shields.io/crates/v/pastebin.svg)](https://crates.io/crates/pastebin)
[![docs.rs](https://docs.rs/pastebin/badge.svg)](https://docs.rs/pastebin)
[![travis-ci](https://travis-ci.org/mexus/pastebin.svg?branch=master)](https://travis-ci.org/mexus/pastebin)

# Simple pastebin service

## About

A simple multipurpose RESTful storage server library written in
[Rust](https://www.rust-lang.org/) with [Iron](https://github.com/iron/iron)
web framework under the cover.

Please note! This library crate provides only a library (obviously), and a real
server is decoupled into a separate crate
[`pastebind`](https://crates.io/crates/pastebind). Usage information is
provided in that crate as well.

## REST api

To upload data (be it text or a file) simply send it using either a `POST` or a
`PUT` request to `/`. You can additinally specify a file name as a URI segment,
like `/file.txt`. The service will reply with a link that contains ID of the
paste. That address should be used later to manipulate the paste.

To specify an expiry date add a query parameter `expires` to your `POST`
(`PUT`) request with value of a desired expiration date (UTC) in the form of a
unix timestamp, like the following: `?expires=1546300800` for the 1st of
January, 2019 (UTC). If you don't specify the date it will be set to the
server's defaults (default expiration time is passed as a command line argument
to the service application). In order to make a paste to be stored without a
time limit you have to pass a special value `never`, like the following:
`?expires=never`.

To download data send a `GET` request to `/id`, where `id` is a paste ID
obtained on the previous step. Actually it's not like you don't have to
specifically obtain an ID, just use the returned link from the `POST` (`PUT`)
as it is. If the paste has information about its file name the service will
redirect the request to `/id/file-name` so you'll be able to save the file
under the correct name. By the way, if you want to take advantage of this
feature while using `wget` pass `--content-disposition` flag to your command.

You can also optionally provide a desired file name like `/id/file-name` to
your `GET` request.

To delete a paste send a `DELETE` request to `/id`, and the paste will be
deleted (if it exists obviously).

## Performance

To be done.
