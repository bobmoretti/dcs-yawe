//! A silly POC of a way to offload arbitrary tasks to a worker thread
//! (e.g., to serialize them without breaking out `async` machinery)
//! and await their results.

use std::sync::mpsc::{channel, sync_channel, Receiver, RecvError, Sender, TryRecvError};

/// Arbitrary work we can enqueue in a channel and then call in another thread
/// on the receiving end.
pub type PackagedTask<T> = Box<dyn FnOnce(&T) + Send>;

/// Packages jobs into something that can be awaited with a [`Future`]
#[derive(Debug)]
pub struct TaskSender<T> {
    /// Tasks to do.
    ///
    /// By the time they get here, they're type-erased into any closure
    /// we can send.
    todos: Sender<PackagedTask<T>>,
}

/// Something that can be waited on in order to produce a result
/// computed by calls to [`tick()`]
#[derive(Debug)]
pub struct Future<T> {
    // This could maybe be something simpler, but IIRC
    // std::sync::mpsc (now with crossbeam!) provides a pretty lightweight
    // channel implementation if it's a bounded chnanel of size 1.
    rx: Receiver<T>,
}

impl<T> Future<T> {
    /// Returns the result, or a [`RecvError`] if this outlived the
    /// [`TaskSender`]
    pub fn wait(self) -> Result<T, RecvError> {
        self.rx.recv()
    }
}

impl<ArgT> TaskSender<ArgT> {
    pub fn new() -> (Self, Receiver<PackagedTask<ArgT>>) {
        let (todos, rx) = channel::<PackagedTask<ArgT>>();
        (Self { todos }, rx)
    }

    /// Push work onto the worker thread,
    /// returning a [`Future`] that can await the result.
    pub fn send<T, F>(&self, fun: F) -> Future<T>
    where
        T: Send + 'static,
        F: FnOnce(&ArgT) -> T + Send + 'static,
    {
        let (tx, rx) = sync_channel(1);
        // Type erasure! Put our closure in a closure that sends the result.
        let erased = move |x: &_| {
            // We don't care if the future gets its result;
            // if send() fails there's nothing we can do from here.
            let _ = tx.send(fun(x));
        };
        // We should always be able to send to the worker unless it crashed
        self.todos
            .send(Box::new(erased))
            .expect("Worker thread hung up");
        Future { rx }
    }

    /// Push work onto the worker thread and immediately wait for it to finish.
    ///
    /// Useful for other threads to serialize work on a worker calling
    /// [`tick()`]
    pub fn run<T, F>(&self, fun: F) -> T
    where
        T: Send + 'static,
        F: FnOnce(&ArgT) -> T + Send + 'static,
    {
        // We shouldn't get an error here since `wait()`
        // only fails if the future outlives the worker,
        // and here we are in the worker.
        self.send(fun).wait().unwrap()
    }
}

/// Run a single packaged task, presumably on a worker thread.
///
/// Return an error if the [`TaskSender`] hung up or there's nothing to do.
pub fn tick<ArgT>(rx: &Receiver<PackagedTask<ArgT>>, arg: ArgT) -> Result<(), RecvError> {
    rx.recv().map(|job| job(&arg))
}

/// Run a single packaged task if one is ready, presumably on a worker thread.
///
/// Return an error if the [`TaskSender`] hung up or there's nothing to do.
pub fn try_tick<ArgT>(rx: &Receiver<PackagedTask<ArgT>>, arg: ArgT) -> Result<(), TryRecvError> {
    rx.try_recv().map(|job| job(&arg))
}
