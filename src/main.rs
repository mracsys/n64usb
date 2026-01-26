use core::time;
use std::{thread::sleep, time::Duration};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[allow(non_upper_case_globals)]
#[allow(non_camel_case_types)]
#[allow(non_snake_case)]
#[allow(dead_code)]
#[allow(unnecessary_transmutes)]
mod flashcart {
    use std::ptr;
    use std::os::raw::c_uchar;
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

    pub struct Header {
        pub datatype: USBDataType,
        pub length: usize,
    }

    pub fn initialize() {unsafe { device_initialize() }}
    pub fn find() -> DeviceError {unsafe { device_find() }}
    pub fn get_cart() -> CartType {unsafe { device_getcart() }}
    pub fn open() -> DeviceError {unsafe { device_open() }}
    pub fn close() -> DeviceError {unsafe { device_close() }}
    pub fn read() -> Result<(Header, Vec<u8>), DeviceError> {
        let mut raw_header: u32 = 0;
        let mut buff_ptr: *mut c_uchar = ptr::null_mut();
        let err = unsafe {
            device_receivedata(&mut raw_header, &mut buff_ptr)
        };
        if err != DeviceError::OK {
            return Err(err);
        }
        if buff_ptr.is_null() {
            return Err(DeviceError::MALLOCFAIL);
        }
        let header = Header {
            datatype: unsafe {std::mem::transmute(raw_header >> 24)},
            length: (raw_header & 0x00FFFFFF) as usize,
        };
        let data = unsafe {
            Vec::from_raw_parts(buff_ptr, header.length, header.length)
        };
        Ok((header, data))
    }
    pub fn write(header: Header, data: Vec<u8>) -> DeviceError {
        unsafe {
            device_senddata(header.datatype, data.as_ptr() as *mut u8, header.length as u32)
        }
    }

    pub fn cart_type_to_str(cart: CartType) -> String {
        String::from(match cart {
            CartType::NONE => "None",
            CartType::_64DRIVE1 => "64Drive HW1",
            CartType::_64DRIVE2 => "64Drive HW2",
            CartType::EVERDRIVE => "Everdrive (X7 or V3)",
            CartType::SC64 => "Summercart64",
            CartType::GOPHER64 => "Gopher64",
        })
    }

    impl DeviceError {
        pub fn value(&self) -> u8 {
            *self as u8
        }
    }
}

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
                let status = flashcart::find();
                if status == flashcart::DeviceError::CARTFINDFAIL {
                    println!("Flashcart disconnected, resetting");
                    flashcart::initialize();
                    State::Searching
                } else if status != flashcart::DeviceError::OK {
                    // Flashcart not found, wait and retry
                    sleep(FOR_ONE_SECOND);
                    State::Searching
                } else {
                    println!("Flashcart found, {}", flashcart::cart_type_to_str(flashcart::get_cart()));
                    println!("Opening connection");
                    State::Opening
                }
            }
            State::Opening => {
                let status = flashcart::open();
                if status != flashcart::DeviceError::OK {
                    println!("Failed to open USB connection to flashcart, retrying");
                    sleep(FOR_ONE_SECOND);
                    State::Opening
                } else {
                    println!("Flashcart USB connection opened");
                    State::WaitForGame
                }
            }
            State::WaitForGame => {
                match flashcart::read() {
                    Ok((header, _)) => {
                        if header.datatype == flashcart::USBDataType::HEARTBEAT {
                            println!("Heartbeat detected");
                            sleep(FOR_ONE_SECOND);
                            //State::WaitForGame
                            let msg = "cmdt".as_bytes().to_vec();
                            let header = flashcart::Header { datatype: flashcart::USBDataType::TEXT, length: msg.len() };
                            println!("Sending cmdt handshake");
                            let status = flashcart::write(header, msg);
                            if status == flashcart::DeviceError::OK {
                                println!("Handshake sent");
                                State::Handshake
                            } else {
                                println!("Failed to send handshake, retrying, {}", status.value());
                                State::WaitForGame
                            }
                        } else {
                            println!("Invalid heartbeat");
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
                match flashcart::read() {
                    Ok((header, data)) => {
                        if header.datatype == flashcart::USBDataType::RAWBINARY {
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
                                let header = flashcart::Header { datatype: flashcart::USBDataType::RAWBINARY, length: msg.len() };
                                println!("Handshake reply received. Repeating protocol version to finalize handshake");
                                let status = flashcart::write(header, msg);
                                if status == flashcart::DeviceError::OK {
                                    println!("Protocol version sent");
                                    State::Idle
                                } else {
                                    println!("Failed to send protocol version, restarting handshake");
                                    State::WaitForGame
                                }
                            }
                        } else {
                            println!("Invalid handshake reply");
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
            State::Idle => {
                let (header, data) = flashcart::read().unwrap_or_else(|_| (flashcart::Header{datatype: flashcart::USBDataType::HEADER, length: 0}, Vec::new()));
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
                    let header = flashcart::Header { datatype: flashcart::USBDataType::RAWBINARY, length: msg.len() };
                    let status = flashcart::write(header, msg);
                    if status == flashcart::DeviceError::OK {
                        self.count += 1;
                    }
                    sleep(FOR_ONE_SECOND);
                    State::Idle
                } else {
                    State::Closing
                }
            }
            State::Closing => {
                let status = flashcart::close();
                if status == flashcart::DeviceError::CLOSEFAIL {
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

    flashcart::initialize();
    let mut worker = Worker { state: State::Searching, count: 0 };
    println!("Started Multiworld Client. Press 'Ctrl+C' to exit.");
    while running.load(Ordering::SeqCst) && !matches!(worker.state, State::Finished) {
        worker = worker.next();
    }

    match worker.state {
        State::Finished => println!("Status: Success - Worker finished its job."),
        _ => {
            println!("Status: Interrupted - Worker was stopped early.");
            let mut status = flashcart::DeviceError::CLOSEFAIL;
            while status != flashcart::DeviceError::OK {
                println!("Closing USB connection");
                status = flashcart::close();
                sleep(FOR_ONE_SECOND);
            }
        },
    }
}
