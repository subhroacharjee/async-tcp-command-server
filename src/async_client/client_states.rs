#[derive(Debug)]
pub enum ClientStates {
    Waiting,
    ReadCommand,
    RunningCommand,
    WriteOutput(String),
    ToBeClosed,
    Close,
    Closed,
}
