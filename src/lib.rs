mod core;
mod empty_message_loop;

pub use crate::core::{run, spawn_local, MessageLoop, MessageLoopWaker};
pub use crate::empty_message_loop::EmptyMessageLoop;
