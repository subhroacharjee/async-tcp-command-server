use std::{
    io::Result,
    sync::{Arc, Mutex, RwLock},
};

use async_server::core::AsyncTcpCommandServer;
use event_loop::core::EventLoop;
use reactor::{core::Reactor, event_listener::EventListener};

pub mod async_client;
pub mod async_server;
pub mod command;
pub mod event_loop;
pub mod reactor;
fn main() -> Result<()> {
    let mut reactor = Arc::new(RwLock::new(Reactor::default()));
    let server = AsyncTcpCommandServer::new("127.0.0.1:7878".to_string(), reactor.clone());
    let mut event_loop = EventLoop::new(reactor.clone());
    event_loop
        .connection_handler_map
        .insert(server.id(), Box::new(server));
    event_loop.run()
}
