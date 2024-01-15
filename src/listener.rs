use std::{any::TypeId, hash::{Hash, Hasher}, sync::Arc, fmt::Debug};
use tokio::sync::Mutex;
use crate::{Event, event::EventHandler};

/// Unique identifier for each Listener
#[derive(Clone, Debug, Eq)]
pub struct ListenerId {
    pub id: usize,
    pub type_id: TypeId,
}

impl PartialEq for ListenerId {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.type_id == other.type_id
    }
}

impl Hash for ListenerId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
        self.type_id.hash(state);
    }
}

#[derive(Clone)]
pub struct Listener<E: Event> {
    pub(crate) id: ListenerId,
    handler: Arc<Mutex<dyn EventHandler<E>>>,
    pub(crate) once: bool,
}

impl<E: Event> Debug for Listener<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Listener").field("id", &self.id).field("once", &self.once).finish()
    }
}

impl<E: Event + Send + Sync + 'static> Listener<E> {
    pub(crate) fn new(handler: impl EventHandler<E> + 'static, once: bool, listener_id: ListenerId) -> Self {
        Self { handler: Arc::new(Mutex::new(handler)), once, id: listener_id }
    }

    pub(crate) async fn call(&self, event: &E) {
        let mut handler = self.handler.lock().await;
        handler.handle_event(event).await;
    }
}
