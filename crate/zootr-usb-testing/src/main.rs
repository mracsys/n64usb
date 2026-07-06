use core::time;
use std::{thread::sleep, time::Duration};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use arrayref::array_ref;

enum State {
    Searching,
    Opening,
    WaitForGame,
    Handshake,
    Idle,
    Closing,
    Finished,
}

enum GameState {
    Unknown,
    InMenu,
    InGame,
}

struct Worker {
    state: State,
    count: u32,
    sent_messages: u32,
    sent_bytes: u32,
    prev_messages: u32,
    item_sent: bool,
    item_recv: bool,
    item_ack: bool,
    game_state: GameState,
    base_frames: Vec<u32>,
    sent_frames: Vec<u32>,
}

trait StateMachine {
    fn next(self) -> Self;
}

const SLEEP_USEC: u64 = 100000;
const SLEEP_DURATION: Duration = time::Duration::from_millis(SLEEP_USEC / 1000);

const ENCODING: [char; 256] = [
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'あ', 'い', 'う', 'え', 'お', 'か',
    'き', 'く', 'け', 'こ', 'さ', 'し', 'す', 'せ', 'そ', 'た', 'ち', 'つ', 'て', 'と', 'な', 'に',
    'ぬ', 'ね', 'の', 'は', 'ひ', 'ふ', 'へ', 'ほ', 'ま', 'み', 'む', 'め', 'も', 'や', 'ゆ', 'よ',
    'ら', 'り', 'る', 'れ', 'ろ', 'わ', 'を', 'ん', 'ぁ', 'ぃ', 'ぅ', 'ぇ', 'ぉ', 'っ', 'ゃ', 'ゅ',
    'ょ', 'が', 'ぎ', 'ぐ', 'げ', 'ご', 'ざ', 'じ', 'ず', 'ぜ', 'ぞ', 'だ', 'ぢ', 'づ', 'で', 'ど',
    'ば', 'び', 'ぶ', 'べ', 'ぼ', 'ぱ', 'ぴ', 'ぷ', 'ぺ', 'ぽ', 'ア', 'イ', 'ウ', 'エ', 'オ', 'カ',
    'キ', 'ク', 'ケ', 'コ', 'サ', 'シ', 'ス', 'セ', 'ソ', 'タ', 'チ', 'ツ', 'テ', 'ト', 'ナ', 'ニ',
    'ヌ', 'ネ', 'ノ', 'ハ', 'ヒ', 'フ', 'ヘ', 'ホ', 'マ', 'ミ', 'ム', 'メ', 'モ', 'ヤ', 'ユ', 'ヨ',
    'ラ', 'リ', 'ル', 'レ', 'ロ', 'ワ', 'ヲ', 'ン', 'ァ', 'ィ', 'ゥ', 'ェ', 'ォ', 'ッ', 'ャ', 'ュ',
    'ョ', 'ガ', 'ギ', 'グ', 'ゲ', 'ゴ', 'ザ', 'ジ', 'ズ', 'ゼ', 'ゾ', 'ダ', 'ヂ', 'ヅ', 'デ', 'ド',
    'バ', 'ビ', 'ブ', 'ベ', 'ボ', 'パ', 'ピ', 'プ', 'ペ', 'ポ', 'ヴ', 'A', 'B', 'C', 'D', 'E',
    'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U',
    'V', 'W', 'X', 'Y', 'Z', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k',
    'l', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z', ' ',
    '┬', '?', '!', ':', '-', '(', ')', '゛', '゜', ',', '.', '/', '�', '�', '�', '�',
    '�', '�', '�', '�', '�', '�', '�', '�', '�', '�', '�', '�', '�', '�', '�', '�',
];

impl StateMachine for Worker {
    fn next(mut self) -> Self {
        self.state = match self.state {
            State::Searching => {
                println!("Searching for flashcart");
                //let status = n64flashcart::find();
                // Wii
                //let status = n64flashcart::connect(0x0403, 0x6001, "BG02Y2TL");
                // SC64
                //let status = n64flashcart::connect(0x0403, 0x6014, "SC64B0PTEH");
                // Everdrive X7
                let status = n64flashcart::connect(0x0403, 0x6001, "A10MQ5HP");
                // Everdrive V3
                //let status = n64flashcart::connect(0x0403, 0x6001, "AC01W748");
                if status == n64flashcart::DeviceError::CARTFINDFAIL {
                    println!("Flashcart disconnected, resetting");
                    n64flashcart::initialize();
                    State::Searching
                } else if status != n64flashcart::DeviceError::OK {
                    // Flashcart not found, wait and retry
                    sleep(SLEEP_DURATION);
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
                    println!("Failed to open USB connection to flashcart, retrying, error code {}", status.value());
                    sleep(SLEEP_DURATION);
                    State::Opening
                } else {
                    println!("Flashcart USB connection opened");
                    State::WaitForGame
                }
            }
            State::WaitForGame => {
                self.game_state = GameState::InGame;
                match n64flashcart::read() {
                    Ok((header, data)) => {
                        if header.datatype == n64flashcart::USBDataType::HANDSHAKE || header.datatype == n64flashcart::USBDataType::HEARTBEAT {
                            println!("Handshake request detected");
                            sleep(SLEEP_DURATION);
                            let msg = "OoTMM\x03".as_bytes().to_vec();
                            
                            let header = n64flashcart::Header { datatype: n64flashcart::USBDataType::HANDSHAKE, length: msg.len() };
                            println!("Sending handshake {}", msg.iter().map(|b| format!("{:02x}", b)).collect::<Vec<String>>().join(" "));
                            let status = n64flashcart::write(header, msg);
                            if status == n64flashcart::DeviceError::OK {
                                println!("Handshake sent");
                                sleep(SLEEP_DURATION);
                                State::Handshake
                            } else {
                                println!("Failed to send handshake, retrying, {}", status.value());
                                State::WaitForGame
                            }
                        } else if header.datatype == n64flashcart::USBDataType::EMPTY {
                            //println!("No data to read while waiting for handshake");
                            sleep(SLEEP_DURATION);
                            State::WaitForGame
                        } else {
                            println!("Invalid handshake, {}, {}", header.datatype.value(), data.iter().map(|b| format!("{:02x}", b)).collect::<Vec<String>>().join(" "));
                            sleep(SLEEP_DURATION);
                            State::WaitForGame
                        }
                    }
                    Err(e) => {
                        println!("Read error while waiting for handshake, {}", e.value());
                        sleep(SLEEP_DURATION);
                        State::WaitForGame
                    }
                }
            }
            State::Handshake => {
                match n64flashcart::read() {
                    Ok((header, data)) => {
                        if header.datatype == n64flashcart::USBDataType::HANDSHAKE {
                            if data.len() < 6 {
                                println!("Invalid handshake reply, restarting handshake");
                                sleep(SLEEP_DURATION);
                                State::WaitForGame
                            } else if data[0] != b'O' || data[1] != b'o' || data[2] != b'T' || data[3] != b'M' || data[4] != b'M' || data[5] != 3 {
                                println!("Invalid handshake reply, restarting handshake");
                                sleep(SLEEP_DURATION);
                                State::WaitForGame
                            } else {
                                //let protocol_version = data[4];
                                //let mut msg = "MW".as_bytes().to_vec();
                                //msg.push(protocol_version);
                                //msg.push(0); // MW_SEND_OWN_ITEMS
                                //msg.push(0); // MW_PROGRESSIVE_ITEMS_ENABLE
                                //let header = n64flashcart::Header { datatype: n64flashcart::USBDataType::RAWBINARY, length: msg.len() };
                                //println!("Handshake reply received. Repeating protocol version to finalize handshake");
                                //let status = n64flashcart::write(header, msg);
                                //if status == n64flashcart::DeviceError::OK {
                                //    println!("Protocol version sent");
                                //    sleep(FOR_ONE_SECOND);
                                    State::Idle
                                //} else {
                                //    println!("Failed to send protocol version, restarting handshake");
                                //    State::WaitForGame
                                //}
                            }
                        } else if header.datatype == n64flashcart::USBDataType::EMPTY {
                            //println!("No data to read while waiting for handshake reply");
                            sleep(SLEEP_DURATION);
                            State::Handshake
                        } else {
                            println!("Invalid handshake reply, type {} data {}", header.datatype.value(), data.iter().map(|b| format!("{:02x}", b)).collect::<Vec<String>>().join(" "));
                            sleep(SLEEP_DURATION);
                            State::WaitForGame
                        }
                    }
                    Err(e) => {
                        println!("Read error while waiting for handshake reply, {}", e.value());
                        sleep(SLEEP_DURATION);
                        State::Handshake
                    }
                }
            }
            State::Idle => {
                let mut reset_connection = false;
                let mut fatal_error = false;
                let mut reading = true;
                while reading {
                match n64flashcart::read() {
                    Ok((header, data)) => {
                        if header.datatype == n64flashcart::USBDataType::RESET {
                            println!("Reset signal received, restarting handshake");
                            reset_connection = true;
                            self.game_state = GameState::Unknown;
                        } else if header.datatype == n64flashcart::USBDataType::HANDSHAKE {
                            println!("Console requested to restart handshake?");
                            //reset_connection = true;
                            //self.game_state = GameState::Unknown;
                        } else if header.datatype == n64flashcart::USBDataType::HEARTBEAT {
                            println!("Received heartbeat from console, ignoring.");
                        } else if header.datatype == n64flashcart::USBDataType::ITEM_GIVEN {
                            let replyheader = n64flashcart::Header { datatype: n64flashcart::USBDataType::ACK_MESSAGE, length: 4 };
                            let _ = n64flashcart::write(replyheader, vec![0u8, 4]);
                            if self.item_sent && !self.item_ack {
                                println!("Item receipt confirmed");
                                self.item_ack = true;
                            } else {
                                println!("Item receipt confirmed without sending an item!!");
                            }
                        } else if header.datatype == n64flashcart::USBDataType::SEND_ITEM && header.length > 0 {
                            let replyheader = n64flashcart::Header { datatype: n64flashcart::USBDataType::ACK_MESSAGE, length: 4 };
                            let _ = n64flashcart::write(replyheader, vec![0u8, 4]);
                            if data[9] != 0x5A {
                                println!("Received item send request: {}", data.iter().map(|b| format!("{:02x}", b)).collect::<Vec<String>>().join(" "));
                                self.item_recv = true;
                            } else {
                                println!("Received echo item send request: {}", data.iter().map(|b| format!("{:02x}", b)).collect::<Vec<String>>().join(" "));
                            }
                        } else if header.datatype == n64flashcart::USBDataType::INGAME_STATE {
                            println!("Received save context state packet. Length: {}", header.length);
                            println!("Raw bytes: {}", data.iter().map(|b| format!("{:02x}", b)).collect::<Vec<String>>().join(" "));
                            self.game_state = GameState::InGame;
                        } else if header.datatype == n64flashcart::USBDataType::SAVE_FILENAME && header.length == 16 {
                            let filename: String = data[1..=8].iter().map(|&b| ENCODING[b as usize]).collect();
                            println!("Received filename state packet, str: {}, raw: {}", filename, data.iter().map(|b| format!("{:02x}", b)).collect::<Vec<String>>().join(" "));
                            self.game_state = GameState::InMenu;
                        } else if header.datatype == n64flashcart::USBDataType::RAWBINARY && header.length == 12 {
                            let data_slice = data.into_boxed_slice();
                            let messages = u32::from_be_bytes(*array_ref![data_slice, 0, 4]);
                            let bytes = u32::from_be_bytes(*array_ref![data_slice, 4, 4]);
                            let frame_time = u32::from_be_bytes(*array_ref![data_slice, 8, 4]);
                            //println!("Messages: {}, Actual: {}, Bytes: {}, Actual: {}, Time: {}", messages, self.sent_messages, bytes, self.sent_bytes, frame_time);
                            if bytes != 6 {
                                match self.prev_messages {
                                    0 => self.base_frames.push(frame_time),
                                    _ => self.sent_frames.push(frame_time),
                                }
                                self.prev_messages = messages;
                                self.sent_bytes -= bytes;
                                self.sent_messages -= messages;
                            }
                            println!("Messages: {}, Remaining: {}, Bytes: {}, Remaining: {}, Time: {}", messages, self.sent_messages, bytes, self.sent_bytes, frame_time);
                        } else if header.length > 0 {
                            println!("Received unknown data from console, ignoring. Type: 0x{}, Length: {}, data: {}", format!("{:02x}", header.datatype.value()), header.length, data.iter().map(|b| format!("{:02x}", b)).collect::<Vec<String>>().join(" "));
                        } else if header.datatype == n64flashcart::USBDataType::EMPTY && header.length == 0 {
                            reading = false;
                        }
                    }
                    Err(e) => {
                        println!("Read error, {}", e.value());
                        match e {
                            n64flashcart::DeviceError::_64D_BADCMP => {
                                fatal_error = true;
                            }
                            _ => {}
                        }
                    }
                }
                }
                match self.game_state {
                    GameState::InGame => {
                        // if self.count < 10000 {
                        //     if self.count % 1000 == 0 {
                        //         println!("Waiting...{}", 10 - self.count / 1000);
                        //     }
                        //     self.count += 1;
                        // } else if self.count == 10000 {
                        //     println!("Giving Light Arrows");
                        //     let msg: Vec<u8> = vec![
                        //         0x01, // playerFrom
                        //         0x01, // playerTo
                        //         0x00, // game
                        //         0x00, // zero padding
                        //         0x01, // key (ovType)
                        //         0x55, // key (sceneId)
                        //         0x00, // key (roomId)
                        //         0x00, // key (id)
                        //         0x00, // gi high
                        //         0x5A, // gi low
                        //         0x00, // flags high
                        //         0x00, // flags low
                        //     ];
                        //     let header = n64flashcart::Header { datatype: n64flashcart::USBDataType::SEND_ITEM, length: msg.len() };
                        //     let status = n64flashcart::write(header, msg);
                        //     if status == n64flashcart::DeviceError::OK {
                        //         self.count += 1;
                        //         self.item_sent = true;
                        //     }
                        // } else {
                            //if self.count % 50 == 0 {
                                let base_angle = 3.1415 * (self.count % 500) as f32 / 250.0;
                                let radius = 104.0;
                                for j in 0..128 {
                                    let i = j % 16;
                                    let angle = base_angle + (i as f32) * 2.0 * 3.1415 / 16.0;
                                    let x_part = (f32::sin(angle) * radius).round();
                                    let z_part = (f32::cos(angle) * radius).round();
                                    let x = (0xFF08 as u16).wrapping_add(x_part as i16 as u16); // 208 == 0xD0
                                    let z = (0xFFF8 as u16).wrapping_add(z_part as i16 as u16); // 208 == 0xD0
                                    let mut msg: Vec<u8> = vec![
                                        0x00, // frameCount
                                        0x00, // frameCount
                                        0x00, // frameCount
                                        0x00, // frameCount
                                        0x00, // sceneKey high
                                        0x55, // sceneKey low
                                        0x00, // x high
                                        0x00, // x low
                                        0x00, // y high
                                        0x00, // y low
                                        0x00, // z high
                                        0x00, // z low
                                        0x00, // clientId high
                                        0x02 + (i as u8), // clientId low
                                    ];
                                    msg[0..4].copy_from_slice(&self.count.to_be_bytes());
                                    msg[6..8].copy_from_slice(&x.to_be_bytes());
                                    msg[10..12].copy_from_slice(&z.to_be_bytes());
                                    //let msg = vec![0u8; 14];
                                    let header = n64flashcart::Header { datatype: n64flashcart::USBDataType::PLAYER_POS, length: msg.len() };
                                    let msglen = msg.len() as u32;
                                    let status = n64flashcart::write(header, msg);
                                    if status == n64flashcart::DeviceError::OK {
                                        self.sent_messages += 1;
                                        self.sent_bytes += msglen;
                                    } else {
                                        println!("Position send error: {}", status.value());
                                    }
                                }
                            //}
                            self.count += SLEEP_USEC as u32 / 2000;
                        //}
                    }
                    _ => {
                        //println!("Waiting for game to start");
                        self.count = 0;
                    }
                }
                let next_state;
                // if self.item_sent && self.item_recv && self.item_ack {
                //     println!("All test conditions satisfied");
                //     next_state = State::Closing;
                // } else
                if reset_connection {
                    next_state = State::WaitForGame;
                } else if fatal_error {
                    next_state = State::Closing;
                } else {
                    next_state = State::Idle;
                }
                sleep(SLEEP_DURATION);
                next_state
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

    let devices = n64flashcart::list();
    for d in devices {
        println!("{:?}", d);
    }

    let mut worker = Worker {
        state: State::Searching,
        count: 0,
        item_sent: false,
        item_ack: false,
        item_recv: false,
        game_state: GameState::InGame,
        sent_messages: 0,
        sent_bytes: 0,
        prev_messages: 0,
        base_frames: Vec::default(),
        sent_frames: Vec::default(),
    };
    println!("Started Multiworld Client. Press 'Ctrl+C' to exit.");
    while running.load(Ordering::SeqCst) && !matches!(worker.state, State::Finished) {
        worker = worker.next();
    }

    let base_avg = worker.base_frames.iter().map(|&x| x as u64).sum::<u64>() as f64 / (worker.base_frames.len() as f64 * 1000.0);
    let sent_avg = worker.sent_frames.iter().map(|&x| x as u64).sum::<u64>() as f64 / (worker.sent_frames.len() as f64 * 1000.0);
    println!("Base frametime: {}, Messaging frametime: {}, Delta (ms): {}", base_avg, sent_avg, sent_avg - base_avg);

    match worker.state {
        State::Finished => println!("Status: Success - Worker finished its job."),
        _ => {
            println!("Status: Interrupted - Worker was stopped early.");
            let mut status = n64flashcart::DeviceError::CLOSEFAIL;
            while status != n64flashcart::DeviceError::OK {
                println!("Closing USB connection");
                status = n64flashcart::close();
                sleep(SLEEP_DURATION);
            }
        },
    }
}
