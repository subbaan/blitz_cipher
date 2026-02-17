use gilrs::Gilrs;
use std::time::Duration;

fn main() {
    let mut gilrs = Gilrs::new().expect("Failed to init gilrs");

    println!("Move the stick and press buttons. Ctrl+C to quit.\n");

    for (_id, gp) in gilrs.gamepads() {
        println!("Found: \"{}\" (id: {:?})", gp.name(), _id);
    }
    println!();

    loop {
        while let Some(event) = gilrs.next_event() {
            println!("{:?}", event);
        }
        std::thread::sleep(Duration::from_millis(16));
    }
}
