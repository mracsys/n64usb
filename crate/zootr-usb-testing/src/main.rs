use core::time;
use std::{thread::sleep, time::Duration};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

enum State {
    Searching,
    Opening,
    WaitForGame,
    Handshake,
    Idle,
    Closing,
    Finished,
}

struct Worker {
    state: State,
    count: u32,
}

trait StateMachine {
    fn next(self) -> Self;
}

const FOR_ONE_SECOND: Duration = time::Duration::from_secs(1);

impl StateMachine for Worker {
    fn next(mut self) -> Self {
        self.state = match self.state {
            State::Searching => {
                println!("Searching for flashcart");
                let status = n64flashcart::find();
                if status == n64flashcart::DeviceError::CARTFINDFAIL {
                    println!("Flashcart disconnected, resetting");
                    n64flashcart::initialize();
                    State::Searching
                } else if status != n64flashcart::DeviceError::OK {
                    // Flashcart not found, wait and retry
                    sleep(FOR_ONE_SECOND);
                    State::Searching
                } else {
                    println!("Flashcart found, {}", n64flashcart::cart_type_to_str(n64flashcart::get_cart()));
                    println!("Opening connection");
                    State::Opening
                }
            }
            State::Opening => {
                let status = n64flashcart::open();
                if status != n64flashcart::DeviceError::OK {
                    println!("Failed to open USB connection to flashcart, retrying");
                    sleep(FOR_ONE_SECOND);
                    State::Opening
                } else {
                    println!("Flashcart USB connection opened");
                    State::WaitForGame
                }
            }
            State::WaitForGame => {
                match n64flashcart::read() {
                    Ok((header, data)) => {
                        if header.datatype == n64flashcart::USBDataType::HEARTBEAT {
                            println!("Heartbeat detected");
                            sleep(FOR_ONE_SECOND);
                            //State::WaitForGame
                            let msg = "cmdt".as_bytes().to_vec();
                            
                            let header = n64flashcart::Header { datatype: n64flashcart::USBDataType::TEXT, length: msg.len() };
                            println!("Sending cmdt handshake");
                            let status = n64flashcart::write(header, msg);
                            if status == n64flashcart::DeviceError::OK {
                                println!("Handshake sent");
                                sleep(FOR_ONE_SECOND);
                                State::Handshake
                            } else {
                                println!("Failed to send handshake, retrying, {}", status.value());
                                State::WaitForGame
                            }
                        } else {
                            println!("Invalid heartbeat, {}, {}", header.datatype.value(), String::from_utf8(data).unwrap());
                            sleep(FOR_ONE_SECOND);
                            State::WaitForGame
                        }
                    }
                    Err(_) => {
                        println!("No data to read while waiting for heartbeat");
                        sleep(FOR_ONE_SECOND);
                        State::WaitForGame
                    }
                }
            }
            State::Handshake => {
                match n64flashcart::read() {
                    Ok((header, data)) => {
                        if header.datatype == n64flashcart::USBDataType::RAWBINARY {
                            if data.len() < 16 {
                                println!("Invalid handshake reply, restarting handshake");
                                sleep(FOR_ONE_SECOND);
                                State::WaitForGame
                            } else if data[0] != b'O' || data[1] != b'o' || data[2] != b'T' || data[3] != b'R' {
                                println!("Invalid handshake reply, restarting handshake");
                                sleep(FOR_ONE_SECOND);
                                State::WaitForGame
                            } else {
                                let protocol_version = data[4];
                                let mut msg = "MW".as_bytes().to_vec();
                                msg.push(protocol_version);
                                msg.push(0); // MW_SEND_OWN_ITEMS
                                msg.push(0); // MW_PROGRESSIVE_ITEMS_ENABLE
                                let header = n64flashcart::Header { datatype: n64flashcart::USBDataType::RAWBINARY, length: msg.len() };
                                println!("Handshake reply received. Repeating protocol version to finalize handshake");
                                let status = n64flashcart::write(header, msg);
                                if status == n64flashcart::DeviceError::OK {
                                    println!("Protocol version sent");
                                    State::Idle
                                } else {
                                    println!("Failed to send protocol version, restarting handshake");
                                    State::WaitForGame
                                }
                            }
                        } else {
                            println!("Invalid handshake reply, {}", header.datatype.value());
                            sleep(FOR_ONE_SECOND);
                            State::WaitForGame
                        }
                    }
                    Err(_) => {
                        println!("No data to read while waiting for handshake reply");
                        sleep(FOR_ONE_SECOND);
                        State::Handshake
                    }
                }
            }
            State::Idle => {
                let (header, data) = n64flashcart::read().unwrap_or_else(|_| (n64flashcart::Header{datatype: n64flashcart::USBDataType::HEADER, length: 0}, Vec::new()));
                if header.length == 16 && u32::from_be_bytes(data.try_into().unwrap()) == 0x01000000 {
                    println!("Reset signal received, restarting handshake");
                    State::WaitForGame
                } else if header.length > 0 {
                    println!("Received data from console, ignoring.");
                    sleep(FOR_ONE_SECOND);
                    State::Idle
                } else if self.count < 10 {
                    println!("Waiting...{}", 10 - self.count);
                    sleep(FOR_ONE_SECOND);
                    self.count += 1;
                    State::Idle
                } else if self.count == 10 {
                    println!("Giving Light Arrows");
                    let msg: Vec<u8> = vec![0x02, 0x00, 0x5A];
                    let header = n64flashcart::Header { datatype: n64flashcart::USBDataType::RAWBINARY, length: msg.len() };
                    let status = n64flashcart::write(header, msg);
                    if status == n64flashcart::DeviceError::OK {
                        self.count += 1;
                    }
                    sleep(FOR_ONE_SECOND);
                    State::Idle
                } else {
                    State::Closing
                }
            }
            State::Closing => {
                let status = n64flashcart::close();
                if status == n64flashcart::DeviceError::CLOSEFAIL {
                    println!("Failed to close USB connection to flashcart, retrying");
                    State::Closing
                } else {
                    println!("Flashcart USB connection closed");
                    State::Finished
                }
            }
            State::Finished => {
                State::Finished
            }
        };
        self
    }
}

fn main() {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    }).expect("Error setting Ctrl-C handler");

    n64flashcart::initialize();
    n64flashcart::set_protocol(n64flashcart::ProtocolVer::VERSION2);
    let mut worker = Worker { state: State::Searching, count: 0 };
    println!("Started Multiworld Client. Press 'Ctrl+C' to exit.");
    while running.load(Ordering::SeqCst) && !matches!(worker.state, State::Finished) {
        worker = worker.next();
    }

    match worker.state {
        State::Finished => println!("Status: Success - Worker finished its job."),
        _ => {
            println!("Status: Interrupted - Worker was stopped early.");
            let mut status = n64flashcart::DeviceError::CLOSEFAIL;
            while status != n64flashcart::DeviceError::OK {
                println!("Closing USB connection");
                status = n64flashcart::close();
                sleep(FOR_ONE_SECOND);
            }
        },
    }
}
