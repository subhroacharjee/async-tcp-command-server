use mio::net::{TcpListener, TcpStream};

pub enum ServerStates {
    Waiting,
    Accepting(TcpStream),
    Close(TcpListener),
    Closed,
}
