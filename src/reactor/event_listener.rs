use std::io::Result;

use mio::event::Event;

pub trait EventListener {
    fn id(&self) -> usize;
    fn name(&self) -> String;
    fn poll(&mut self) -> Result<()>;
    fn handle_event(&mut self, event: &Event);
}
