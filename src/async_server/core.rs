use std::{
    io::{self, Result},
    os::fd::AsRawFd,
    rc::Rc,
    sync::{Arc, Mutex, RwLock},
};

use mio::{
    net::{TcpListener, TcpStream},
    Interest,
};

use crate::{
    async_client::core::AsyncClientHandler,
    reactor::{core::Reactor, event_listener::EventListener},
};

use super::server_states::ServerStates;

pub struct AsyncTcpCommandServer {
    reactor: Arc<RwLock<Reactor>>,
    listener: Rc<TcpListener>,
    fd: usize,
    state: Option<ServerStates>,
}

impl AsyncTcpCommandServer {
    pub fn new(addr: String, reactor: Arc<RwLock<Reactor>>) -> AsyncTcpCommandServer {
        let mut listener = TcpListener::bind(addr.parse().unwrap()).unwrap();
        let fd = listener.as_raw_fd() as usize;
        {
            let mut reactor = reactor.write().unwrap();
            reactor
                .register(fd, &mut listener, Some(Interest::READABLE))
                .unwrap();
        }

        AsyncTcpCommandServer {
            reactor,
            listener: Rc::new(listener),
            fd,
            state: None,
        }
    }

    fn handle_new_connection(&mut self, client: TcpStream) -> Result<()> {
        let client_fd = client.as_raw_fd() as usize;
        // create a new client handler and add it to the reactot add connection method
        let mut reactor = self.reactor.write().unwrap();
        let client_handler = AsyncClientHandler::new(client, self.reactor.clone());
        reactor.add_new_connection(client_fd, client_handler);
        self.state.replace(ServerStates::Waiting);
        reactor.schedule(self.id());
        Ok(())
    }

    fn close_connection(&mut self, mut listner: TcpListener) -> Result<()> {
        let fd = self.fd;
        let mut reactor = self.reactor.write().unwrap();
        reactor.remove_old_connection(fd, &mut listner);
        Ok(())
    }
}

impl EventListener for AsyncTcpCommandServer {
    fn id(&self) -> usize {
        self.fd
    }

    fn name(&self) -> String {
        format!(
            "AsyncTcpCommandServer tcp://{}",
            self.listener.local_addr().unwrap()
        )
    }

    fn poll(&mut self) -> std::io::Result<()> {
        println!("server poll is called");
        if let Some(state) = self.state.take() {
            match state {
                ServerStates::Accepting(client) => {
                    self.handle_new_connection(client)?;
                }
                ServerStates::Close(listner) => {
                    self.close_connection(listner)?;
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn handle_event(&mut self, event: &mio::event::Event) {
        if event.is_readable() {
            if let Some((client, addr)) = match self.listener.accept() {
                Ok((connection, addr)) => Some((connection, addr)),
                Err(err) if err.kind() == io::ErrorKind::WouldBlock => {
                    self.state.replace(ServerStates::Waiting);
                    None
                }
                Err(err) => {
                    println!("err in listner: {}", err);
                    let listner_clone = self.listener.clone();
                    self.state
                        .replace(ServerStates::Close(Rc::try_unwrap(listner_clone).unwrap()));
                    None
                }
            } {
                println!("recieved new connection from {}", addr);
                self.state.replace(ServerStates::Accepting(client));
            }

            {
                let fd = self.id();
                let mut reactor = self.reactor.write().unwrap();
                reactor.schedule(fd);
            }
        }
    }
}
