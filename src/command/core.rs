use std::{
    sync::{Arc, Mutex, RwLock},
    thread::JoinHandle,
};

use mio::Waker;

use crate::{
    async_client::client_states::ClientStates,
    command::{echo::Echo, ping::Ping},
    reactor::core::Reactor,
};

pub trait Command {
    fn can_process(&mut self, raw_cmd: String) -> bool;
    fn run(
        &mut self,
        raw_cmd: String,
        fd: usize,
        state: Arc<Mutex<Option<ClientStates>>>,
        waker: Arc<Waker>,
        reactor: Arc<RwLock<Reactor>>,
    ) -> JoinHandle<()>;
}

fn registered_commands() -> Vec<Box<dyn Command>> {
    vec![Box::new(Ping {}), Box::new(Echo {})]
}

pub fn get_and_run_cmd(
    fd: usize,
    raw_cmd: String,
    state: Arc<Mutex<Option<ClientStates>>>,
    waker: Arc<Waker>,
    reactor: Arc<RwLock<Reactor>>,
) -> Option<JoinHandle<()>> {
    let mut commands = registered_commands();
    for cmd in commands.iter_mut() {
        if cmd.as_mut().can_process(raw_cmd.to_string()) {
            return Some(cmd.run(raw_cmd, fd, state, waker, reactor));
        }
    }
    None
}
