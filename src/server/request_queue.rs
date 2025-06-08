use std::{
    collections::VecDeque,
    sync::{Arc, Condvar, Mutex},
    thread,
    time::Duration,
};

use crate::request::Request;

use super::handlers::{HandlerCallError, HandlerRegistry, RequestDispatcher};

pub struct RequestQueueOptions {
    n_threads: usize,
    timeout: Duration,
}

/// Adapts to the number of cores available to the program
impl Default for RequestQueueOptions {
    fn default() -> Self {
        Self {
            n_threads: thread::available_parallelism().map_or(4, |res| res.get().div_ceil(2)),
            timeout: Duration::new(10, 0),
        }
    }
}

pub struct RequestQueue {
    threads: Vec<thread::JoinHandle<()>>,
    timeout: Duration,
    // FIXME: using my own implementation of a synchronised queue
    // will not be as performant as using a more mature abstraction.
    // This should be swapped out for `crossbeam_channel::unbounded`.
    // I chose to implement my own version to learn about synchronisation
    // and borrow-checking in Rust
    reqs: Arc<SynchronisedQueue<Request>>,
}

impl RequestQueue {
    pub fn new<D: RequestDispatcher + Send + Sync + 'static>(
        dispatcher: D,
        opts: RequestQueueOptions,
    ) -> Self {
        let req_queue = Arc::new(SynchronisedQueue::new());
        let wrapped_dispatcher = Arc::new(dispatcher);

        let mut threads = Vec::new();
        for _ in 0..opts.n_threads {
            let queue_ref = Arc::clone(&req_queue);
            let dispatcher_ref = Arc::clone(&wrapped_dispatcher);
            let t = thread::spawn(move || loop {
                let req = queue_ref.pop();
                match dispatcher_ref.dispatch(&req) {
                    Ok(res) => {
                        // TODO: pass on response to response queue,
                    }
                    Err(e) => {
                        // TODO: pass to error handler function to generate the correct
                        // response status code + message and push to response queue
                    }
                }
            });
            threads.push(t);
        }
        Self {
            threads,
            timeout: opts.timeout,
            reqs: req_queue,
        }
    }

    pub fn enqueue(&mut self, request: Request) {
        self.reqs.push(request)
    }
}

struct SynchronisedQueue<T: Send> {
    signal: Condvar,
    data: Mutex<VecDeque<T>>,
}

impl<T: Send> SynchronisedQueue<T> {
    pub fn new() -> Self {
        Self {
            signal: Condvar::new(),
            data: Mutex::new(VecDeque::new()),
        }
    }

    pub fn with_capacity(size: usize) -> Self {
        Self {
            signal: Condvar::new(),
            data: Mutex::new(VecDeque::with_capacity(size)),
        }
    }

    pub fn push(&self, x: T) {
        let mut data = self.data.lock().unwrap();
        data.push_back(x);
        self.signal.notify_one();
    }

    pub fn pop(&self) -> T {
        let mut data = self.data.lock().unwrap();
        loop {
            if let Some(x) = data.pop_front() {
                return x;
            }
            data = self.signal.wait(data).unwrap();
        }
    }

    pub fn len(&self) -> usize {
        let data = self.data.lock().unwrap();
        data.len()
    }

    pub fn is_empty(&self) -> bool {
        let data = self.data.lock().unwrap();
        data.is_empty()
    }
}
