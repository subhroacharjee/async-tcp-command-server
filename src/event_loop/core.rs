use std::{
    collections::HashMap,
    io::Result,
    sync::{Arc, RwLock},
};

use mio::Events;

use crate::reactor::{core::Reactor, event_listener::EventListener};

pub struct EventLoop {
    pub connection_handler_map: HashMap<usize, Box<dyn EventListener>>,
    pub reactor: Arc<RwLock<Reactor>>,
}

impl EventLoop {
    pub fn new(reactor: Arc<RwLock<Reactor>>) -> EventLoop {
        EventLoop {
            connection_handler_map: HashMap::new(),
            reactor,
        }
    }

    pub fn run(&mut self) -> Result<()> {
        loop {
            // get all the scheduled tasks and poll for them
            self.poll_for_scheduled_tasks()?;
            // handle new connections
            self.handle_new_connections()?;
            // handle old connections
            self.handle_dead_connections()?;
            // wait for io events and run events for them
            self.wait_for_events()?;
        }
    }

    fn poll_for_scheduled_tasks(&mut self) -> Result<()> {
        loop {
            if let Some(id) = {
                let mut reactor = self.reactor.write().unwrap();
                reactor.tasks.pop()
            } {
                println!("scheduled task for Fd {}", id);
                if let Some(handler) = self.connection_handler_map.get_mut(&id) {
                    match handler.poll() {
                        Ok(_) => {}
                        Err(err) => {
                            println!("FD {} has failed with error: {}", id, err);
                        }
                    }
                }
            }
            let res = {
                let reactor = self.reactor.read().unwrap();
                reactor.tasks.is_empty()
            };
            if res {
                break;
            }
        }

        Ok(())
    }

    fn handle_new_connections(&mut self) -> Result<()> {
        while let Some((fd, handler)) = {
            let mut reactor = self.reactor.write().unwrap();
            reactor.new_source.pop()
        } {
            self.connection_handler_map.insert(fd, handler);
            self.connection_handler_map.get_mut(&fd).unwrap().poll()?;
        }
        println!("all new connections are handled");
        Ok(())
    }

    fn handle_dead_connections(&mut self) -> Result<()> {
        let mut reactor = self.reactor.write().unwrap();
        while let Some(fd) = reactor.old_source.pop() {
            if let Some(handler) = self.connection_handler_map.remove(&fd) {
                drop(handler);
            }
        }
        println!("all old connections are handled");
        Ok(())
    }

    fn wait_for_events(&mut self) -> Result<()> {
        println!("waiting for i/o");
        let mut events = Events::with_capacity(1024);
        {
            let mut reactor = self.reactor.write().unwrap();
            reactor.wait(&mut events).unwrap();
        }
        for ev in events.iter() {
            println!("events {:?}", ev);

            if let Some(handler) = self.connection_handler_map.get_mut(&ev.token().0) {
                handler.handle_event(ev);
            }
        }

        Ok(())
    }
}
