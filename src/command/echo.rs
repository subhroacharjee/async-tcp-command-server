use std::thread;

use crate::async_client::client_states::ClientStates;

use super::core::Command;

pub struct Echo {}

impl Command for Echo {
    fn can_process(&mut self, raw_cmd: String) -> bool {
        raw_cmd.to_lowercase().starts_with("echo")
    }

    fn run(
        &mut self,
        raw_cmd: String,
        fd: usize,
        state: std::sync::Arc<
            std::sync::Mutex<Option<crate::async_client::client_states::ClientStates>>,
        >,
        waker: std::sync::Arc<mio::Waker>,
        reactor: std::sync::Arc<std::sync::RwLock<crate::reactor::core::Reactor>>,
    ) -> std::thread::JoinHandle<()> {
        let waker = waker.clone();
        thread::spawn(move || {
            println!(
                "Worker thread {:?} spawned for ping command",
                thread::current().id()
            );
            let echo_re = regex::Regex::new(r"(?i)^echo(.*)").unwrap();
            let mut echo_val = String::new();

            if let Some(val) = echo_re.captures(&raw_cmd) {
                echo_val = val[1].trim().to_string() + "\n";
            }
            state
                .lock()
                .unwrap()
                .replace(ClientStates::WriteOutput(echo_val));

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
