use std::{
    thread::{self, sleep},
    time,
};

fn main() {
    let mut handlers = vec![];
    for i in 0..10 {
        let t = thread::spawn(move || {
            let start = time::Instant::now();
            println!("thread: {}", i);
            let duration = time::Instant::now().duration_since(start);
            sleep(std::time::Duration::from_secs(2));
            println!("thread: {} took: {:?}", i, duration.as_micros())
        });
        handlers.push(t);
    }
    for h in handlers {
        h.join().unwrap();
    }
}
