use std::{
    collections::HashSet,
    io::{self, Result},
    sync::Arc,
    time::Duration,
};

use mio::{event::Source, Events, Interest, Poll, Waker};

use super::event_listener::EventListener;

pub struct Reactor {
    pub existing_tokens: HashSet<usize>,
    pub poller: Poll,
    pub tasks: Vec<usize>,
    pub new_source: Vec<(usize, Box<dyn EventListener + Send + Sync>)>,
    pub old_source: Vec<usize>,
    pub waker: Option<Arc<Waker>>,
}

impl Default for Reactor {
    fn default() -> Reactor {
        let poller = Poll::new().unwrap();
        Reactor {
            existing_tokens: HashSet::new(),
            poller,
            tasks: vec![],
            new_source: vec![],
            old_source: vec![],
            waker: None,
        }
    }
}

impl Reactor {
    /// Returns the events that has been recieved by the poller;
    /// This method will poll for events and it will return Ok(events) only when events  is
    /// present.
    /// In case polling returns and empty event list it will stay inside the loop.
    /// # Errors
    ///
    /// This function will return an error if .
    /// mio::Poll::poll method returns any error except for Interrupted ;
    pub fn wait(&mut self, events: &mut Events) -> Result<()> {
        while events.is_empty() {
            match self.poller.poll(events, Some(Duration::from_nanos(20))) {
                Ok(_) => {
                    if !self.tasks.is_empty() {
                        break;
                    }
                }
                Err(err) => {
                    if err.kind() == io::ErrorKind::Interrupted {
                        // in case notify is called we break out of the waiting loop and do one
                        // iteration of event loop
                        break;
                    } else {
                        return Err(err);
                    }
                }
            }
        }
        Ok(())
    }

    /// Takes a listener or TcpStream as source along with it;s Fd and Interest is None.
    /// If no interest is passed explictly we will add READABLE and WRITABLE interest to the event
    /// so that it works for both.
    ///
    /// Suggested: use READABLE interest for server and keep None for clients
    ///
    /// # Errors
    ///
    /// This function will return an error if
    /// register register will throw error
    pub fn register(
        &mut self,
        fd: usize,
        source: &mut impl Source,
        interest: Option<Interest>,
    ) -> Result<()> {
        if !self.existing_tokens.contains(&fd) {
            let interest = interest.unwrap_or(Interest::READABLE.add(Interest::WRITABLE));
            self.existing_tokens.insert(fd);

            return self
                .poller
                .registry()
                .register(source, mio::Token(fd), interest);
        }
        Ok(())
    }

    /// It takes a source and then it removes from the poller instance;
    ///
    /// # Errors
    ///
    /// This function will return an error if deregister fn return any error
    pub fn unregister(&mut self, source: &mut impl Source) -> Result<()> {
        self.poller.registry().deregister(source)
    }

    /// It takes an Fd and add it to schduler;
    pub fn schedule(&mut self, id: usize) {
        self.tasks.push(id);
    }

    pub fn add_new_connection<E>(&mut self, fd: usize, handler: E)
    where
        E: EventListener + Send + Sync + 'static,
    {
        self.new_source.push((handler.id(), Box::new(handler)));
        self.schedule(fd);
    }

    pub fn remove_old_connection(&mut self, fd: usize, source: &mut impl Source) {
        self.unregister(source).unwrap();
        self.existing_tokens.remove(&fd);
        self.old_source.push(fd);
    }

    fn init_waker(&mut self) {
        match self.waker {
            None => {
                self.waker.replace(Arc::new(
                    mio::Waker::new(self.poller.registry(), mio::Token(0)).unwrap(),
                ));
            }
            _ => {}
        };
    }

    pub fn get_waker_for_fd(&mut self) -> Arc<Waker> {
        if let Some(waker) = self.waker.clone() {
            return waker;
        } else {
            self.init_waker();
            return self.get_waker_for_fd();
        }
    }
}
