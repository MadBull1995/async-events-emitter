//! # Event Emitter Library
//!
//! `async-events-emitter` is a Rust library providing an implementation of an event handling system.
//! It allows users to define custom events and handlers, and emit events that are processed asynchronously.
//!
//! ## Usage
//! ```
//! use async_events_emitter::*;
//! use async_trait::async_trait;
//! 
//! // Define your events
//! #[derive(Debug, Clone)]
//! struct MyEvent;
//! 
//! // Impl the EventHandler trait
//! #[async_trait]
//! impl EventHandler<MyEvent> for MyHandler {
//!     async fn handle_event(&self, event: MyEvent) {
//!         // TODO: Process and handle your event async
//!     }
//! }
//! 
//! let ee = EventEmitter::new();
//! // Init the handler
//! let handler = EventHandler;
//! // Attach the handler to your event
//! ee.on::<MyEvent>(handler);
//! // Now emit the event
//! ee.emit(MyEvent);
//! ```
//! 
//! ## Features
//! - Define custom events.
//! - Register asynchronous event handlers.
//! - Emit events to be handled by registered handlers.

extern crate tokio;

mod event;
mod listener;
mod event_emitter;

// Re-exporting the main structs and traits
pub use self::{event::{Event, EventError, EventHandler}, event_emitter::EventEmitter};

#[cfg(test)]
mod test {
    use std::{sync::{Arc, atomic::{AtomicBool, Ordering}}, time::Instant};

    use async_trait::async_trait;
    use tokio::sync::Mutex;

    use super::*;


    // Example of custom event
    #[derive(Debug, Clone)]
    struct CustomEvent1 {
        data: String,
    }

    #[derive(Debug, Clone)]
    struct CustomEvent2 {
        data: usize
    }

    impl Event for CustomEvent1 {}
    impl Event for CustomEvent2 {}


    #[derive(Debug, Clone)]
    struct CounterHandler {
        handle_count: Arc<tokio::sync::Mutex<usize>>,
    }

    #[derive(Debug, Clone)]
    struct SharedDataHandler1 {
        data: Arc<Mutex<Vec<String>>>,
    }

    #[derive(Debug, Clone)]
    struct SharedDataHandler2 {
        data: Arc<Mutex<Vec<String>>>,
    }

    #[derive(Debug)]
    struct UnsubscribeHandler {
        received_flag: Arc<AtomicBool>,
    }

    #[derive(Debug, Clone)]
    struct SimpleHandler;

    #[derive(Debug, Clone)]
    struct ComplexHandler {
        data: Arc<tokio::sync::Mutex<String>>,
    }

    #[async_trait]
    impl EventHandler<CustomEvent1> for SimpleHandler {
        async fn handle_event(&mut self, event: &CustomEvent1) {
            println!("[SimpleHandler] Handling CustomEvent1 with data: {}", event.data);
        }
    }

    #[async_trait]
    impl EventHandler<CustomEvent2> for SimpleHandler {
        async fn handle_event(&mut self, event: &CustomEvent2) {
            println!("[SimpleHandler] Handling CustomEvent2 with data: {}", event.data);
        }
    }

    #[async_trait]
    impl EventHandler<CustomEvent1> for SharedDataHandler1 {
        async fn handle_event(&mut self, event: &CustomEvent1) {
            let mut data = self.data.lock().await;
            
            data.push(event.data.clone() + " from handler1");
        }
    }

    #[async_trait]
    impl EventHandler<CustomEvent1> for SharedDataHandler2 {
        async fn handle_event(&mut self, event: &CustomEvent1) {
            let mut data = self.data.lock().await;
            
            data.push(event.data.clone() + " from handler2");
        }
    }

    #[async_trait]
    impl EventHandler<CustomEvent1> for UnsubscribeHandler {
        async fn handle_event(&mut self, event: &CustomEvent1) {
            println!("[UnsubscribeHandler] Handling CustomEvent1 with data: {}", event.data);
            let flag = &self.received_flag;
            flag.store(true, Ordering::SeqCst);
        }
    }

    #[async_trait]
    impl EventHandler<CustomEvent1> for CounterHandler {
        async fn handle_event(&mut self, _event: &CustomEvent1) {
            // println!("[CounterHandler] Handling CustomEvent1 with data: {}", event.data);
            let mut counter = self.handle_count.lock().await;
            *counter += 1;
        }
    }

    #[async_trait]
    impl EventHandler<CustomEvent2> for CounterHandler {
        async fn handle_event(&mut self, _event: &CustomEvent2) {
            // println!("[CounterHandler] Handling CustomEvent2 with data: {}", event.data);
            let mut counter = self.handle_count.lock().await;
            *counter += 1;
        }
    }

    #[async_trait]
    impl EventHandler<CustomEvent1> for ComplexHandler {
        async fn handle_event(&mut self, event: &CustomEvent1) {
            // Async handling logic
            println!("[ComplexHandler] Handling CustomEvent1 with data: {}", event.data);
            // Lock the mutex to get mutable access to the data
            let mut data = self.data.lock().await;

            // Perform your mutations here
            // For example, appending some information from the event
            *data = format!("{} - Event Handled", *data);

        }
    }

    #[async_trait]
    impl EventHandler<CustomEvent2> for ComplexHandler {
        async fn handle_event(&mut self, event: &CustomEvent2) {
            // Async handling logic
            println!("[ComplexHandler] Handling CustomEvent2 with data: {}", event.data);
        }
    }

    #[tokio::test]
    async fn test_event_once() {
        let mut event_emitter = EventEmitter::new();
        let handler = CounterHandler {
            handle_count: Arc::new(Mutex::new(0))
        };
        event_emitter.once::<CustomEvent1>(handler.clone());
        let event = CustomEvent1 {
            data: "Should be triggered once!".to_string()
        };
        event_emitter.emit(event.clone());
        event_emitter.emit(event);
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        assert_eq!(event_emitter.listeners_count(), 0);
        let handler_count =  handler.handle_count.lock().await;
        assert_eq!(handler_count.to_owned(), 1);

    }

    #[tokio::test]
    async fn test_unsubscribe_functionality() {
        let mut event_emitter = EventEmitter::new();
        let received_flag = Arc::new(AtomicBool::new(false));
        let received_flag_clone = received_flag.clone();

        let handler = UnsubscribeHandler {
            received_flag: received_flag_clone
        };

        let listener_id = event_emitter.on::<CustomEvent1>(handler);
        event_emitter.remove_listener(listener_id);

        event_emitter.emit(CustomEvent1 { data: "Test".to_string() });
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await; // Allow time for handlers to process

        assert_eq!(received_flag.load(Ordering::SeqCst), false);
    }

    #[tokio::test]
    async fn test_multiple_subscribers() {
        let mut event_emitter = EventEmitter::new();
        let shared_data = Arc::new(tokio::sync::Mutex::new(vec![]));

        let handler1_data = shared_data.clone();
        let handler1 = SharedDataHandler1 {
            data: handler1_data
        };

        let handler2_data = shared_data.clone();
        let handler2 = SharedDataHandler2 {
            data: handler2_data
        };

        event_emitter.on::<CustomEvent1>(handler1);
        event_emitter.on::<CustomEvent1>(handler2);

        event_emitter.emit(CustomEvent1 { data: "Hello".to_string() });
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await; // Allow time for handlers to process

        let data = shared_data.lock().await;
        assert_eq!(data.len(), 2);
        assert!(data.contains(&"Hello from handler1".to_string()));
        assert!(data.contains(&"Hello from handler2".to_string()));
    }


    #[tokio::test]
    async fn test_high_load() {
        let event_emitter = Arc::new(Mutex::new(EventEmitter::new()));
        let event_count = 1000;
        let handler_count = 100;
        let counter = Arc::new(Mutex::new(0));
        // Add a large number of handlers
        for _ in 0..handler_count {
            let emitter = event_emitter.clone();
            let handler = CounterHandler {
                handle_count: Arc::clone(&counter)
            };
            emitter.lock().await.on::<CustomEvent1>(handler);
        }
        let start_time = Instant::now();
        // Emit a large number of events
        let event = CustomEvent1 { data: "Some value".to_string() };
        for _ in 0..event_count {
            let cloned_ee = Arc::clone(&event_emitter);
            let cloned_e = event.clone();
            tokio::spawn(async move {
                cloned_ee.lock().await.emit(cloned_e);
            });
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(335)).await;
        let duration = start_time.elapsed();
        let total_events = event_count * handler_count;
        
        // Ensure all events have been processed
        let handled_events = counter.lock().await;
        assert_eq!(*handled_events, total_events, "Not all events were handled correctly");
        println!("Time taken to process events {}: {:?}", total_events, duration);
    }

    #[tokio::test]
    async fn test_event_emitter() {
        let mut event_emitter = EventEmitter::new();
        let handler = ComplexHandler {
            data: Arc::new(tokio::sync::Mutex::new(String::new())),
        };

        event_emitter.on::<CustomEvent1>(handler.clone());

        let listener_id = event_emitter.on::<CustomEvent2>(handler.clone());

        let event = CustomEvent1 { data: "Hello World".to_string() };
        let event2 = CustomEvent2 { data: 1 };

        event_emitter.emit(event.clone());
        event_emitter.remove_listener(listener_id);
        event_emitter.emit(event2);
        
        // let mut new_ee = event_emitter;

        // new_ee.emit(event)
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        let data = handler.data.lock().await;
        assert_eq!(data.to_owned(), " - Event Handled")

    }

    #[tokio::test]
    async fn test_listeners() {
        let mut ee = EventEmitter::with_capacity(1);
        let handler = SimpleHandler;
        let handler_1 = ComplexHandler {
            data: Arc::new(Mutex::new("".to_string())),
        };
        ee.on::<CustomEvent1>(handler.clone());
        ee.on::<CustomEvent1>(handler_1);
        ee.on::<CustomEvent2>(handler);
        let listeners = ee.get_event_listeners::<CustomEvent1>().unwrap();
        assert_eq!(listeners.len() , 2);
    }
}
