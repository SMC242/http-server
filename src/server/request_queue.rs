use std::{
    collections::VecDeque,
    io::Error as IoError,
    sync::{Arc, Condvar, Mutex},
    thread,
    time::{Duration, SystemTime},
};

use log::{error, info};

use crate::request::Request;

use super::handlers::{DispatcherError, RequestDispatcher};

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

enum ThreadPoolMessage<T> {
    /// Work to pass to the `ThreadPool`'s callback
    Work(T),
    /// Shutdown signal
    Die,
}

pub trait ThreadPool<I>
where
    I: Send + Sync + 'static,
{
    fn enqueue(&mut self, to_process: I);
    /// Send the signal to stop processing further jobs
    fn shutdown(&mut self);

    fn spawn_all<F>(
        &mut self,
        callback: F,
        work: Arc<SynchronisedQueue<ThreadPoolMessage<I>>>,
        n_threads: usize,
    ) -> Result<Vec<thread::JoinHandle<()>>, IoError>
    where
        F: Fn(I) + Send + Sync + Clone + 'static,
    {
        assert!(n_threads > 0, "{n_threads} is an invalid number of threads");

        let mut threads = Vec::with_capacity(n_threads);
        for worker_num in 0..n_threads {
            let work_ref = Arc::clone(&work);
            let cb = callback.clone();
            let th = thread::Builder::new().spawn(move || loop {
                let message = work_ref.pop();

                match message {
                    ThreadPoolMessage::Work(job) => {
                        let start_time = SystemTime::now();
                        cb(job);
                        info!(
                            "Job processed by worker {0} finished in {1} ms",
                            worker_num,
                            start_time
                                .elapsed()
                                .expect("The clock didn't change during the job")
                                .as_millis()
                        );
                    }
                    ThreadPoolMessage::Die => {
                        info!("Shutting down worker {worker_num}");
                        break;
                    }
                }
            });

            threads.push(th?);
        }

        Ok(threads)
    }
}

pub struct RequestQueue {
    threads: Option<Vec<thread::JoinHandle<()>>>,
    // FIXME: using my own implementation of a synchronised queue
    // will not be as performant as using a more mature abstraction.
    // This should be swapped out for `crossbeam_channel::unbounded`.
    // I chose to implement my own version to learn about synchronisation
    // and borrow-checking in Rust
    reqs: Arc<SynchronisedQueue<ThreadPoolMessage<Request>>>,
}

impl ThreadPool<Request> for RequestQueue {
    fn enqueue(&mut self, to_process: Request) {
        self.reqs.push(ThreadPoolMessage::Work(to_process))
    }

    fn shutdown(&mut self) {
        if let Some(threads) = self.threads.take() {
            // This is a hack around the fact that there are no "close"
            // semantics for my queue. Instead, I send a message to each worker
            // to join
            for _ in 0..threads.len() {
                self.reqs.push(ThreadPoolMessage::Die);
            }

            for th in threads {
                th.join().expect("The thread should join");
            }
        }
    }
}

impl RequestQueue {
    pub fn new<D: RequestDispatcher + Send + Sync + 'static>(
        dispatcher: Arc<D>,
        opts: RequestQueueOptions,
    ) -> Result<Self, IoError> {
        let req_queue = Arc::new(SynchronisedQueue::with_capacity(opts.n_threads));
        let mut instance = Self {
            reqs: Arc::clone(&req_queue),
            threads: None,
        };

        let dispatcher_ref = Arc::clone(&dispatcher);

        let threads = ThreadPool::spawn_all(
            &mut instance,
            move |req| {
                let response = dispatcher_ref.dispatch(req).unwrap_or_else(|err| {
                    err.into_response()
                        .build()
                        .expect("A valid handler call error response should be produced")
                });
                info!("Produced response: {response}");
                let _ = response
                    .send()
                    .inspect_err(|err| error!("Error occurred when sending response {err}"));
            },
            req_queue,
            opts.n_threads,
        );

        threads.map(|ts| {
            instance.threads = Some(ts);
            instance
        })
    }
}

impl Drop for RequestQueue {
    fn drop(&mut self) {
        self.shutdown();
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
