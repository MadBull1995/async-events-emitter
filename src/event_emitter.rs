use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use dashmap::DashMap;

use crate::{
    event::EventHandler,
    listener::{Listener, ListenerId},
    Event,
};

type EventsMap = Arc<DashMap<TypeId, HashMap<ListenerId, Box<dyn Any + Send + Sync>>>>;

/// The [`EventEmitter`] struct is the core of the event handling system.
/// It allows registration of event handlers and emits events to these handlers.
///
/// # Examples
/// ```no_compile
/// use async_events_emitter::{EventEmitter, Event, EventHandler};
/// use async_trait::async_trait;
/// 
/// #[derive(Debug, Clone)]
/// pub struct MyCustomEvent;
///
/// #[derive(Debug, Clone)]
/// pub struct MyEventHandler;
/// 
/// impl Event for MyCustomEvent {};
///
/// #[async_trait]
/// impl EventHandler<MyCustomEvent> for MyEventHandler {
///     async fn handle_event(&mut self, event: MyCustomEvent) {
///         // Handle event data here..
///         println!("{:?}", event)
///     }
/// }
/// 
/// // Create a new EventEmitter
/// let mut event_emitter = EventEmitter::new();
/// let my_event_handler = MyEventHandler;
///
/// // Register event handlers
/// event_emitter.on::<MyCustomEvent>(my_event_handler);
///
/// // Emit an event
/// event_emitter.emit(MyCustomEvent);
/// ```
pub struct EventEmitter {
    events: EventsMap,
    next_id: AtomicUsize,
}

impl Default for EventEmitter {
    fn default() -> Self {
        Self::new()
    }
}

impl EventEmitter {
    /// Creates a new instance of the `EventEmitter`.
    ///
    /// This function initializes the `EventEmitter` with empty event handlers. It's ideal for 
    /// starting an event-driven application where you'll be adding handlers dynamically.
    pub fn new() -> Self {
        Self {
            events: Arc::new(DashMap::new()),
            next_id: AtomicUsize::new(0),
        }
    }

    /// Creates a new instance of the `EventEmitter` with a specified capacity.
    ///
    /// This variant of the constructor allows you to specify an initial capacity for event listeners,
    /// which can be useful for performance optimization in scenarios where a large number of listeners
    /// are expected.
    ///
    /// # Arguments
    /// * `capacity`: The initial capacity for storing event listeners.
    ///
    /// # Examples
    /// ```
    /// let mut event_emitter = EventEmitter::with_capacity(100);
    /// ```
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            events: Arc::new(DashMap::with_capacity(capacity)),
            next_id: AtomicUsize::new(0),
        }
    }
    /// Registers a handler for a specific event type.
    ///
    /// The on method allows you to register a handler that will be called every time an event of a specific type is emitted. This is useful for setting up listeners that should persist and handle all occurrences of an event.
    /// # Type Parameters
    /// * E: The type of the event to listen for. This type must implement the Event trait.
    ///
    /// # Arguments
    /// * handler: An implementation of the EventHandler<E> trait. This handler will be called for every emitted event of type E.
    ///
    /// # Returns
    /// Returns a ListenerId which can be used to remove the listener at a later time with the off method.
    ///
    /// # Examples
    /// ```
    /// use async_events_emitter::*;
    /// 
    /// #[derive(Debug, Clone)]
    /// struct MyCustomEvent;
    /// 
    /// let mut event_emitter = EventEmitter::new();
    /// event_emitter.on::<MyCustomEvent>(my_event_handler);
    /// ```
    pub fn on<E: Event + 'static>(
        &mut self,
        handler: impl EventHandler<E> + 'static,
    ) -> ListenerId {
        self.add_listener(handler, false)
    }
    
    /// Registers a handler for a specific event type, but only for the next occurrence.
    ///
    /// This method is similar to on, but the handler will be automatically unregistered after it handles an event once.
    ///
    /// # Type Parameters
    /// * E: The type of the event to listen for.
    ///
    /// # Arguments
    /// * handler: The handler for the event, which will be called only once.
    ///
    /// # Returns
    /// Returns a ListenerId, which can be used to prematurely remove the listener if needed.
    ///
    /// # Examples
    /// ```
    /// use async_events_emitter::*;
    /// 
    /// #[derive(Debug, Clone)]
    /// struct MyCustomEvent;
    /// 
    /// let mut event_emitter = EventEmitter::new();
    /// event_emitter.once::<MyCustomEvent>(my_event_handler);
    /// ```
    pub fn once<E: Event + 'static>(
        &mut self,
        handler: impl EventHandler<E> + 'static,
    ) -> ListenerId {
        self.add_listener(handler, true)
    }

    /// Emits an event, triggering all registered handlers for its type.
    ///
    /// This method is used to emit an event of type E, which will be handled by all the registered handlers for this type.
    ///
    /// # Type Parameters
    /// * E: The type of the event being emitted. Must implement Event and Clone and be Send and 'static.
    ///
    /// # Arguments
    /// * event: The event instance to emit.
    pub fn emit<E: Event + 'static>(&self, event: E)
    where
        E: Clone + Send + 'static, // Ensure E is Clone and Send
    {
        let listeners = {
            self.events
                .get(&TypeId::of::<E>())
                .map(|listeners_for_type| {
                    listeners_for_type
                        .iter()
                        .filter_map(|(_, listener)| listener.downcast_ref::<Listener<E>>())
                        .cloned() // Clone each listener
                        .collect::<Vec<_>>() // Collect into a Vec
                })
        };
        let mut to_remove = Vec::new();

        if let Some(ref listeners) = listeners {
            for listener in listeners.clone() {
                let event_clone = event.clone(); // Clone event for each listener
                if listener.once {
                    to_remove.push(listener.id.clone());
                }
                tokio::spawn(async move {
                    listener.call(&event_clone).await
                    // if let Err(e) =  {
                    //     eprintln!("Error handling event: {:?}", e);
                    // }
                });
            }
        }
        drop(listeners); // Important: Release the lock before modifying the collection

        // Remove 'once' listeners
        if !to_remove.is_empty() {
            if let Some(mut listeners_for_type) = self.events.get_mut(&TypeId::of::<E>()) {
                for id in to_remove {
                    listeners_for_type.remove(&id);
                }
            }
        }

    }

    /// An alias for [`EventEmitter::remove_listener()`] Removes a specific listener based on its ListenerId.
    ///
    /// This method is used to unsubscribe a previously registered event handler. The handler will no longer receive event notifications.
    ///
    /// # Arguments
    /// * listener_id: The ListenerId of the handler to be removed. This ID is obtained when registering a handler using on or once.
    pub fn off<E: Event + 'static>(&mut self, listener_id: ListenerId) where E: Clone + Send + 'static {
        self.remove_listener(listener_id)
    }

    /// Removes a listener from the internal events map based on its ListenerId.
    ///
    /// This internal method provides the core functionality for unregistering an event handler. It is used by the public off method.
    ///
    /// # Arguments
    /// * listener_id: The ListenerId of the listener to be removed.
    pub fn remove_listener(&self, listener_id: ListenerId) {
        if let Some(mut listeners_for_type) = self.events.get_mut(&listener_id.type_id) {
            listeners_for_type.remove(&listener_id);
        }
    }

    /// Retrieves all listeners for a specific event type.
    ///
    /// This method returns a list of all registered listeners for a given event type E. It's useful for debugging or introspection purposes.
    ///
    /// # Type Parameters
    /// * E: The type of the event.
    ///
    /// # Returns
    /// Returns an Option<Vec<Listener<E>>> containing all listeners for the event type E. Returns None if there are no listeners for this event type.
    pub fn get_event_listeners<E: Event + 'static>(&self) -> Option<Vec<Listener<E>>> where E: Clone + Send + 'static {
        let event_listeners = {
            self.events.get(&TypeId::of::<E>()).map(|listeners_of_type| {
                listeners_of_type
                    .iter()
                    .filter_map(|(_, listener)| listener.downcast_ref::<Listener<E>>())
                    .cloned() // Clone each listener
                    .collect::<Vec<_>>() // Collect into a Vec
            })
        };
        event_listeners
    }

    pub fn listeners_count(&self) -> usize {
        // let mut count = 0;
        // for key in self.events.iter().into_iter() {

        //     count += self.events.get(&key).unwrap().len();
        // }
        todo!();
    }

    pub fn event_listeners_count<E: Event + 'static>(&self) -> Option<usize>
    where
        E: Clone + Send + 'static, // Ensure E is Clone and Send
    {
        self.events.get(&TypeId::of::<E>()).map(|listeners| listeners.len())
    }

    fn add_listener<E: Event + 'static>(&mut self, handler: impl EventHandler<E> + 'static, once: bool) -> ListenerId {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let type_id = TypeId::of::<E>();
        let listener_id = ListenerId { id, type_id };
        let listener = Listener::new(handler, once, listener_id.clone());
        self.events
            .entry(type_id)
            .or_default()
            .insert(listener.id.clone(), Box::new(listener));
        listener_id
    }
}
