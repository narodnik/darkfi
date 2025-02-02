use std::{
    net::{SocketAddr, TcpListener},
    sync::Arc,
};

use log::*;
use smol::{Async, Executor};

use crate::{
    system::{StoppableTask, StoppableTaskPtr, Subscriber, SubscriberPtr, Subscription},
    Error, Result,
};

use super::{Channel, ChannelPtr};

/// Atomic pointer to Acceptor class.
pub type AcceptorPtr = Arc<Acceptor>;

/// Create inbound socket connections.
pub struct Acceptor {
    channel_subscriber: SubscriberPtr<Result<ChannelPtr>>,
    task: StoppableTaskPtr,
}

impl Acceptor {
    /// Create new Acceptor object.
    pub fn new() -> Arc<Self> {
        Arc::new(Self { channel_subscriber: Subscriber::new(), task: StoppableTask::new() })
    }
    /// Start accepting inbound socket connections. Creates a listener to start
    /// listening on a local socket address. Then runs an accept loop in a new
    /// thread, erroring if a connection problem occurs.
    pub fn start(
        self: Arc<Self>,
        accept_addr: SocketAddr,
        executor: Arc<Executor<'_>>,
    ) -> Result<()> {
        let listener = Self::setup(accept_addr)?;

        // Start detached task and return instantly
        self.accept(listener, executor);

        Ok(())
    }

    /// Stop accepting inbound socket connections.
    pub async fn stop(&self) {
        // Send stop signal
        self.task.stop().await;
    }

    /// Start receiving network messages.
    pub async fn subscribe(self: Arc<Self>) -> Subscription<Result<ChannelPtr>> {
        self.channel_subscriber.clone().subscribe().await
    }

    /// Start listening on a local socket address.
    fn setup(accept_addr: SocketAddr) -> Result<Async<TcpListener>> {
        let listener = match Async::<TcpListener>::bind(accept_addr) {
            Ok(listener) => listener,
            Err(err) => {
                error!("Bind listener failed: {}", err);
                return Err(Error::OperationFailed)
            }
        };
        let local_addr = match listener.get_ref().local_addr() {
            Ok(addr) => addr,
            Err(err) => {
                error!("Failed to get local address: {}", err);
                return Err(Error::OperationFailed)
            }
        };
        info!("Listening on {}", local_addr);

        Ok(listener)
    }

    /// Run the accept loop in a new thread and error if a connection problem
    /// occurs.
    fn accept(self: Arc<Self>, listener: Async<TcpListener>, executor: Arc<Executor<'_>>) {
        self.task.clone().start(
            self.clone().run_accept_loop(listener),
            |result| self.handle_stop(result),
            Error::ServiceStopped,
            executor,
        );
    }

    /// Run the accept loop.
    async fn run_accept_loop(self: Arc<Self>, listener: Async<TcpListener>) -> Result<()> {
        loop {
            let channel = self.tick_accept(&listener).await?;
            self.channel_subscriber.notify(Ok(channel)).await;
        }
    }

    /// Handles network errors. Panics if error passes silently, otherwise
    /// broadcasts the error.
    async fn handle_stop(self: Arc<Self>, result: Result<()>) {
        match result {
            Ok(()) => panic!("Acceptor task should never complete without error status"),
            Err(err) => {
                // Send this error to all channel subscribers
                let result = Err(err);
                self.channel_subscriber.notify(result).await;
            }
        }
    }

    /// Single attempt to accept an incoming connection. Stops after one
    /// attempt.
    async fn tick_accept(&self, listener: &Async<TcpListener>) -> Result<ChannelPtr> {
        let (stream, peer_addr) = match listener.accept().await {
            Ok((s, a)) => (s, a),
            Err(err) => {
                error!("Error listening for connections: {}", err);
                return Err(Error::ServiceStopped)
            }
        };
        info!("Accepted client: {}", peer_addr);

        let channel = Channel::new(stream, peer_addr).await;
        Ok(channel)
    }
}
