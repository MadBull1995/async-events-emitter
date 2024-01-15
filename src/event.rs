use std::any::{Any, TypeId};
use async_trait::async_trait;


pub trait Event: Any + Send + Sync {
    fn type_id(&self) -> TypeId {
        TypeId::of::<Self>()
    }
}

#[derive(Debug)]
pub enum EventError {
    // Define various kinds of errors that might occur
    HandlerError(String),
}

// Define an async trait for handlers
#[async_trait]
pub trait EventHandler<E: Event + Send + Sync + 'static>: Send + Sync {
    async fn handle_event(&mut self, event: &E);
}