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
  - This is less efficient than using a [MPSC channel](https://doc.rust-lang.org/std/sync/mpsc/index.html) like most sources recommend
- Support for arbitary route handlers via the `Handler` trait

## Planned features

- Middleware support: should be a small change to `RequestQueue` as `Handler` already supports it. See [issue #2](https://github.com/SMC242/http-server/issues/2)
- IDN support: see [issue #3](https://github.com/SMC242/http-server/issues/3)
- HTTP 2 support: see [issue #4](https://github.com/SMC242/http-server/issues/4)
- TLS support: see [issue #5](https://github.com/SMC242/http-server/issues/5)
- HTTP 3 support: the interfaces have been written with this in mind (HTTP 3 uses QUIC instead of TCP as the transport protocol). See [issue #8](https://github.com/SMC242/http-server/issues/8)
- Concurrency improvements: move request parsing from the main thread to the worker threads and use non-blocking TCP sockets
  - This will [drastically increase the maximum concurrent requests](https://stackoverflow.com/a/10654365)
  - See [issue #9](https://github.com/SMC242/http-server/issues/9)
