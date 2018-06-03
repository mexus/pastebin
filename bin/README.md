# Simple pastebin server

[![crates.io](https://img.shields.io/crates/v/pastebind.svg)](https://crates.io/crates/pastebind)
[![travis-ci](https://travis-ci.org/mexus/pastebin.svg?branch=master)](https://travis-ci.org/mexus/pastebin)

## About

A simple multipurpose RESTful storage server written in
[Rust](https://www.rust-lang.org/). It uses [MongoDB](https://www.mongodb.com/)
as a storage backend, [Iron](https://github.com/iron/iron) web framework to do
the web stuff and [highlight.js](https://highlightjs.org/) to do synax
highlighting.

This server is wrapping up a [`pastebin`](https://crates.io/crates/pastebin)
library.

## Running the service

Build (`cargo build` or `cargo build --release`) or install the binary (`cargo
install pastebind`). Then simply launch the executable (there's just one
executable generated) with `--help` flag to see the options. Basically you have
to specify mongodb connection options, path to the `html` (ans `sh`) templates
(`templates` folder in the server's repo) and server external address.

Currently a clean exit is not supported, so just kill the process when you want
to stop it.

## User experience

There are two ways how the service could be used: via the REST api (and command
line tools that implement it) or via your browser.

The only diffence comes when you download a paste. If you download it using some
command line tools, the data (the paste) will be passed as it is. But for
browsers the situation is different: if the paste is considered to be a textual
one (plain text, bash script, c++ code, …) a fancy HTML5 page with a [syntax
hightlighter](https://highlightjs.org/) will be presented to you.

**NOTICE** Please note that browser is detected by its
[user-agent](https://en.wikipedia.org/wiki/User_agent#Use_in_HTTP), so if for
some reason you have disabled reporting of the user agent in your browser the
service will consider you as the REST api user and won't provide a fancy output
for your GET requests (related to the pastes only, the submission form will work
as expected anyhow).

### REST api

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

### Command line interface

One can utilize the REST api of the service by using some simple command-line
tools like `curl` or `wget`. A convenience script is provided by the service,
to download it send a `GET` request on the `/paste.sh` URL. Or you can grab it
from the git repo: [paste.sh](templates/paste.sh.tera), but don't forget to
replace a `{{prefix}}` placeholder with the website http address (like
`https://example.com`).

### Via browser

The main page — `/` — represents a text upload form. To upload a whole file,
especially a binary one, I would advise to use a CLI file uploader.

A *readme* page is available at `/readme` (there's also a link on the `/` page).

By the way, `GET` requests are handled differently for browsers and command
line clients (detected by
[user-agent](https://en.wikipedia.org/wiki/User_agent)): if you ask for a paste
from your command line the paste will be provided as-it-is, i.e. its binary
representation, while a nice page will be rendered for text-like pastes if you
access the page from your browser.

## Performance

To be done.

## Limitations

As for now, there is a hard limit of **15 megabytes** on any incoming data. This
limitations comes from a [BSON document
size](https://docs.mongodb.com/manual/reference/limits/) with some reserve for
extra data.

While it is technically possible to store larger data chunks in a MongoDB using
a [GridFS](https://docs.mongodb.com/manual/core/gridfs/) it has not been
implemented in this project yet. But it is on the plan :)
