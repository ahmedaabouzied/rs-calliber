use tokio::runtime::{Builder, Runtime};

/// Tasker is a manager of asynchronous tasks.
#[derive(Debug)]
pub struct Tasker {
    rt: Runtime,
}

impl Tasker {
    pub fn new() -> Self {
        let rt = Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .unwrap();
        Self { rt }
    }

    pub fn spawn<T>(&mut self, t: T)
    where
        T: Send + std::future::Future + 'static,
        T::Output: Send + 'static,
    {
        let _ = self.rt.spawn(t);
    }
}
