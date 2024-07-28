use std::{
    sync::{Arc, RwLock},
    thread,
};

use mio::Waker;

use crate::{async_client::client_states::ClientStates, reactor::core::Reactor};

use super::core::Command;

pub struct Ping {}

impl Command for Ping {
    fn can_process(&mut self, raw_cmd: String) -> bool {
        raw_cmd.to_lowercase().starts_with("ping")
    }

    fn run(
        &mut self,
        _raw_cmd: String,
        fd: usize,
        state: std::sync::Arc<
            std::sync::Mutex<Option<crate::async_client::client_states::ClientStates>>,
        >,
        waker: std::sync::Arc<Waker>,
        reactor: Arc<RwLock<Reactor>>,
    ) -> std::thread::JoinHandle<()> {
        let waker = waker.clone();
        thread::spawn(move || {
            println!(
                "Worker thread {:?} spawned for ping command",
                thread::current().id()
            );
            state
                .lock()
                .unwrap()
                .replace(ClientStates::WriteOutput("+PONG\t\n".to_string()));

            waker.clone().wake().unwrap();
            println!("deadlock after this?");

            {
                let mut reactor = reactor.write().unwrap();
                reactor.schedule(fd);
            }
            waker.clone().wake().unwrap();

            println!(
                "Worker thread {:?} spawned for ping command finished",
                thread::current().id()
            );
        })
    }
}
