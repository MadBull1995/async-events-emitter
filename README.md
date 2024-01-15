# async-events-emitter

`async-events-emitter` is a Rust library providing an asynchronous event handling system. It allows users to define custom events and handlers, emit events, and process them asynchronously.

## Features

- Define custom events.
- Register asynchronous event handlers.
- Emit events to be handled by registered handlers.

## Installation

Add `async-events-emitter` to your `Cargo.toml`:

```toml
[dependencies]
async-events-emitter = "0.1.0
```

## Usage


## Usage

Here's a quick example to get you started:

```rust
use async_events_emitter::*;
use async_trait::async_trait;

// Define your custom event
#[derive(Debug, Clone)]
struct MyEvent;

// Implement the EventHandler trait
#[async_trait]
impl EventHandler<MyEvent> for MyHandler {
    async fn handle_event(&self, event: MyEvent) {
        // Process and handle your event asynchronously
    }
}

// Usage
#[tokio::main]
async fn main() {
    let mut ee = EventEmitter::new();
    let handler = MyHandler;

    // Attach the handler to your event
    ee.on::<MyEvent>(handler);

    // Emit the event
    ee.emit(MyEvent);
}
```

## License
This library is distributed under the terms of Apache-2.0. See [LICENSE](LICENSE) for details.

---

Created with `</>` by Amit Shmulevitch.