use std::{
    collections::VecDeque,
    io::Error as IoError,
    sync::{Arc, Condvar, Mutex},
    thread,
    time::Duration,
};

use crate::request::Request;

use super::handlers::RequestDispatcher;

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

pub trait ThreadPool<I, O, F>
where
    I: Send + Sync,
    O: Send + Sync,
    F: Fn(I) -> O + Send + Sync + 'static,
{
    fn enqueue(&mut self, to_process: I);

    fn spawn_all(
        &mut self,
        mut callback: F,
        work: Arc<SynchronisedQueue<I>>,
        n_threads: usize,
        timeout: Duration,
    ) -> Result<Vec<thread::JoinHandle<()>>, IoError> {
        let mut threads = Vec::with_capacity(n_threads);
        for _ in 0..n_threads {
            let work_ref = Arc::clone(&work);
            let th = thread::Builder::new().spawn(move || loop {
                let job = work_ref.pop();
                callback(job);
            });

            threads.push(th?);
        }

        Ok(threads)
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
        dispatcher: Arc<D>,
        opts: RequestQueueOptions,
    ) -> Self {
        let req_queue = Arc::new(SynchronisedQueue::with_capacity(opts.n_threads));

        let mut threads = Vec::new();
        for _ in 0..opts.n_threads {
            let queue_ref = Arc::clone(&req_queue);
            let dispatcher_ref = Arc::clone(&dispatcher);
            let t = thread::spawn(move || loop {
                let req = queue_ref.pop();
                match dispatcher_ref.dispatch(&req) {
                    Ok(res) => {
                        // TODO: pass on response to response queue,
                        println!("Response generated: {res:?}");
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
