# http-server

This is a multi-threaded HTTP server that I wrote for the following reasons:

1. To deep-dive on HTTP
2. To learn Rust to have a modern systems-programming language in my toolbelt

It is not intended for production use

## Features

- HTTP versions supported:
  - [x] HTTP 0.9
  - [x] HTTP 1.0
  - [x] HTTP 1.1
- Multi-threading: a thread pool is used
  - I chose to write my own synchronised queue to push myself with the borrow checker and expose myself to Rust's synchronisation primitives
  - This is less efficient than using a [MPSC channel](https://doc.rust-lang.org/std/sync/mpsc/index.html) like the [Rust book does](https://doc.rust-lang.org/book/ch21-02-multithreaded.html#sending-requests-to-threads-via-channels)
- Support for arbitary route handlers via the `Handler` trait

## Planned features

- Middleware support: should be a small change to `RequestQueue` as `Handler` already supports it
- IDN support: currently I am assuming that hostnames are in ASCII
- HTTP 2 support
- TLS support
- HTTP 3 support: the interfaces have been written with this in mind (HTTP 3 uses QUIC instead of TCP as the transport protocol)
