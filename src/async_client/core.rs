use std::{
    io::{self, BufRead, BufReader, Write},
    os::fd::{AsFd, AsRawFd},
    sync::{Arc, Mutex, RwLock},
    thread::JoinHandle,
};

use mio::net::TcpStream;

use crate::{
    command::core::get_and_run_cmd,
    reactor::{core::Reactor, event_listener::EventListener},
};

use super::client_states::ClientStates;

pub struct AsyncClientHandler {
    client: TcpStream,
    reactor: Arc<RwLock<Reactor>>,
    state: Arc<Mutex<Option<ClientStates>>>,
    fd: usize,
    name: String,
    command: Option<JoinHandle<()>>,
    is_writeable: bool,
}

impl AsyncClientHandler {
    pub fn new(client: TcpStream, reactor: Arc<RwLock<Reactor>>) -> AsyncClientHandler {
        let fd = client.as_fd().as_raw_fd() as usize;
        let name = client.peer_addr().unwrap().to_string();
        // let client_rc = Rc::new(RefCell::new(client));
        AsyncClientHandler {
            client,
            reactor,
            state: Arc::new(Mutex::new(None)),
            fd,
            name,
            command: None,
            is_writeable: false,
        }
    }

    pub fn update_state(&mut self, state: ClientStates) {
        self.state.lock().unwrap().replace(state);
    }

    pub fn initalize(&mut self) {
        println!(
            "initalize is called for state {:?}",
            self.state.lock().unwrap()
        );
        self.update_state(ClientStates::Waiting);
        {
            let mut reactor = self.reactor.write().unwrap();

            reactor.register(self.id(), &mut self.client, None).unwrap();

            // reactor.schedule(self.id());
        }
    }
    pub fn read_command(&mut self) {
        let mut schedule_evnt = true;
        println!("read is called for state {:?}", self.state.lock().unwrap());

        let mut buf_reader = BufReader::new(&self.client);
        let mut raw_cmd = String::new();
        match buf_reader.read_line(&mut raw_cmd) {
            Ok(_) => {
                if let Some(handler) = {
                    let waker = self.reactor.write().unwrap().get_waker_for_fd().clone();
                    get_and_run_cmd(
                        self.id(),
                        raw_cmd,
                        self.state.clone(),
                        waker,
                        self.reactor.clone(),
                    )
                } {
                    self.command = Some(handler);
                    self.update_state(ClientStates::RunningCommand);
                    schedule_evnt = false;
                } else {
                    self.update_state(ClientStates::WriteOutput("invalid command".to_string()));
                }
            }
            Err(err) => {
                println!("client {} exiting due to  {}", self.name(), err);
                if let Some(handler) = self.command.take() {
                    handler.join().unwrap();
                }

                self.update_state(ClientStates::ToBeClosed);
            }
        };

        if schedule_evnt {
            {
                let mut reactor = self.reactor.write().unwrap();
                reactor.schedule(self.id());
            }
        }
    }
    pub fn write_command(&mut self, output: String) {
        if let Some(handler) = self.command.take() {
            handler.join().unwrap();
        }
        println!("write is called for state {:?}", self.state.lock().unwrap());

        let buff_len = output.len();
        let mut schedule_evnt = true;

        match self.client.write(&output.to_string().into_bytes()) {
            Ok(n) if n < buff_len => {
                self.update_state(ClientStates::WriteOutput(output));
            }
            Err(err)
                if err.kind() == io::ErrorKind::WouldBlock
                    || err.kind() == io::ErrorKind::Interrupted =>
            {
                self.update_state(ClientStates::WriteOutput(output));
            }
            Ok(_) => {
                self.update_state(ClientStates::Waiting);
                schedule_evnt = false;
            }
            Err(err) => {
                let name = self.name();
                println!("clien {} faced error {}", name, err);
                self.update_state(ClientStates::ToBeClosed);
            }
        }

        if schedule_evnt {
            let mut reactor = self.reactor.write().unwrap();
            reactor.schedule(self.id());
        }
    }
    pub fn to_be_closed(&mut self) {
        if let Some(handler) = self.command.take() {
            handler.join().unwrap();
        }
        println!(
            "to_be_closed is called for state {:?}",
            self.state.lock().unwrap()
        );
        self.update_state(ClientStates::Close);

        {
            let mut reactor = self.reactor.write().unwrap();
            reactor.schedule(self.id());
        }
    }

    pub fn close(&mut self) {
        println!("close is called for state {:?}", self.state.lock().unwrap());
        if let Some(handler) = self.command.take() {
            handler.join().unwrap();
        }

        let mut reactor = self.reactor.write().unwrap();
        reactor.remove_old_connection(self.id(), &mut self.client);
    }
}

impl EventListener for AsyncClientHandler {
    fn id(&self) -> usize {
        self.fd
    }

    fn name(&self) -> String {
        self.name.to_string()
    }

    fn poll(&mut self) -> std::io::Result<()> {
        println!("poll of client");
        println!("current client state {:?}", self.state.lock().unwrap());
        let state = self.state.lock().unwrap().take();
        match state {
            None => {
                self.initalize();
            }
            Some(ClientStates::ReadCommand) => {
                self.read_command();
            }
            Some(ClientStates::WriteOutput(output)) => {
                self.write_command(output);
            }
            Some(ClientStates::ToBeClosed) => {
                self.to_be_closed();
            }
            Some(ClientStates::Close) => {
                self.close();
            }
            Some(state) => {
                self.update_state(state);
            }
        };
        println!("current client state {:?}", self.state.lock().unwrap());
        Ok(())
    }

    fn handle_event(&mut self, event: &mio::event::Event) {
        println!(
            "current client state {:?} from handle_event client",
            self.state.lock().unwrap()
        );
        if event.is_readable() {
            let state = self.state.lock().unwrap().take();
            match state {
                Some(ClientStates::Waiting) => {
                    let name = self.name();
                    println!(
                        "{} recieved an readable event updating state to ReadCommand",
                        name
                    );
                    self.update_state(ClientStates::ReadCommand);
                    {
                        let mut reactor = self.reactor.write().unwrap();
                        reactor.schedule(self.id());
                    }
                }
                Some(state) => self.update_state(state),
                _ => {}
            }
        }
        if event.is_writable() {
            self.is_writeable = true;
        }
        if event.is_write_closed() {
            self.is_writeable = false;
        }
    }
}
