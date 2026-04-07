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

enum GameState {
    Unknown,
    InMenu,
    InGame,
}

struct Worker {
    state: State,
    count: u32,
    item_sent: bool,
    item_recv: bool,
    item_ack: bool,
    dungeon_recv: bool,
    game_state: GameState,
}

trait StateMachine {
    fn next(self) -> Self;
}

const FOR_ONE_SECOND: Duration = time::Duration::from_millis(1);

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
                let status = n64flashcart::find();
                //let status = n64flashcart::connect(0x0403, 0x6001, "BG02Y2TL".to_string());
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
                    println!("Failed to open USB connection to flashcart, retrying, error code {}", status.value());
                    sleep(FOR_ONE_SECOND);
                    State::Opening
                } else {
                    println!("Flashcart USB connection opened");
                    State::WaitForGame
                }
            }
            State::WaitForGame => {
                self.game_state = GameState::Unknown;
                match n64flashcart::read() {
                    Ok((header, data)) => {
                        if header.datatype == n64flashcart::USBDataType::HANDSHAKE || header.datatype == n64flashcart::USBDataType::HEARTBEAT {
                            println!("Handshake request detected");
                            sleep(FOR_ONE_SECOND);
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
                        } else if header.datatype == n64flashcart::USBDataType::EMPTY {
                            //println!("No data to read while waiting for handshake");
                            sleep(FOR_ONE_SECOND);
                            State::WaitForGame
                        } else {
                            println!("Invalid handshake, {}, {}", header.datatype.value(), data.iter().map(|b| format!("{:02x}", b)).collect::<Vec<String>>().join(" "));
                            sleep(FOR_ONE_SECOND);
                            State::WaitForGame
                        }
                    }
                    Err(e) => {
                        println!("Read error while waiting for handshake, {}", e.value());
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
                                    sleep(FOR_ONE_SECOND);
                                    State::Idle
                                } else {
                                    println!("Failed to send protocol version, restarting handshake");
                                    State::WaitForGame
                                }
                            }
                        } else if header.datatype == n64flashcart::USBDataType::EMPTY {
                            //println!("No data to read while waiting for handshake reply");
                            sleep(FOR_ONE_SECOND);
                            State::Handshake
                        } else {
                            println!("Invalid handshake reply, {}", header.datatype.value());
                            sleep(FOR_ONE_SECOND);
                            State::WaitForGame
                        }
                    }
                    Err(e) => {
                        println!("Read error while waiting for handshake reply, {}", e.value());
                        sleep(FOR_ONE_SECOND);
                        State::Handshake
                    }
                }
            }
            State::Idle => {
                let mut reset_connection = false;
                let mut fatal_error = false;
                match n64flashcart::read() {
                    Ok((header, data)) => {
                        if header.length == 16 && header.datatype == n64flashcart::USBDataType::RAWBINARY
                        && matches!(data.as_slice(), [0x01, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]) {
                            println!("Reset signal received, restarting handshake");
                            reset_connection = true;
                            self.game_state = GameState::Unknown;
                        } else if header.datatype == n64flashcart::USBDataType::HANDSHAKE {
                            println!("Console requested to restart handshake?");
                            //reset_connection = true;
                            self.game_state = GameState::Unknown;
                        } else if header.datatype == n64flashcart::USBDataType::HEARTBEAT {
                            println!("Received heartbeat from console, ignoring.");
                        } else if self.item_sent && !self.item_ack {
                            if header.datatype == n64flashcart::USBDataType::RAWBINARY && header.length > 0 {
                                if data[0] == 0x04 {
                                    println!("Item receipt confirmed");
                                    self.item_ack = true;
                                } else {
                                    println!("Invalid data received while waiting for item receipt acknowledgement");
                                }
                            } else if header.datatype == n64flashcart::USBDataType::EMPTY {
                                //println!("No data received while waiting for item receipt acknowledgement");
                            } else {
                                println!("Non-binary data received while waiting for item receipt acknowledgement");
                            }
                        } else if header.datatype == n64flashcart::USBDataType::RAWBINARY && header.length > 0 {
                            if data[0] == 0x03 {
                                println!("Received item send request: {}", data.iter().map(|b| format!("{:02x}", b)).collect::<Vec<String>>().join(" "));
                                self.item_recv = true;
                            } else if data[0] == 0x04 {
                                println!("Item receipt confirmed without sending an item!!");
                            } else if data[0] == 0x05 {
                                println!("Received dungeon info packet: {}", data.iter().map(|b| format!("{:02x}", b)).collect::<Vec<String>>().join(" "));
                                self.dungeon_recv = true;
                            }
                        } else if header.datatype == n64flashcart::USBDataType::INGAME_STATE {
                            println!("Received save context state packet. Length: {}", header.length);
                            println!("Raw bytes: {}", data.iter().map(|b| format!("{:02x}", b)).collect::<Vec<String>>().join(" "));
                            self.game_state = GameState::InGame;
                        } else if header.datatype == n64flashcart::USBDataType::SAVE_FILENAME && header.length == 16 {
                            let filename: String = data[1..=8].iter().map(|&b| ENCODING[b as usize]).collect();
                            println!("Received filename state packet, str: {}, raw: {}", filename, data.iter().map(|b| format!("{:02x}", b)).collect::<Vec<String>>().join(" "));
                            self.game_state = GameState::InMenu;
                        } else if header.length > 0 {
                            println!("Received unknown data from console, ignoring. Type: {}, Length: {}", header.datatype.value(), header.length);
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
                match self.game_state {
                    GameState::InGame => {
                        if self.count < 10000 {
                            if self.count % 1000 == 0 {
                                println!("Waiting...{}", 10 - self.count / 1000);
                            }
                            self.count += 1;
                        } else if self.count == 10000 {
                            println!("Giving Light Arrows");
                            let msg: Vec<u8> = vec![0x02, 0x00, 0x5A];
                            let header = n64flashcart::Header { datatype: n64flashcart::USBDataType::RAWBINARY, length: msg.len() };
                            let status = n64flashcart::write(header, msg);
                            if status == n64flashcart::DeviceError::OK {
                                self.count += 1;
                                self.item_sent = true;
                            }
                        }
                    }
                    _ => {
                        //println!("Waiting for game to start");
                        self.count = 0;
                    }
                }
                let next_state;
                if self.item_sent && self.item_recv && self.item_ack && self.dungeon_recv {
                    println!("All test conditions satisfied");
                    next_state = State::Closing;
                } else if reset_connection {
                    next_state = State::WaitForGame;
                } else if fatal_error {
                    next_state = State::Closing;
                } else {
                    next_state = State::Idle;
                }
                sleep(FOR_ONE_SECOND);
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
    let mut worker = Worker {
        state: State::Searching,
        count: 0,
        item_sent: false,
        item_ack: false,
        item_recv: false,
        dungeon_recv: false,
        game_state: GameState::Unknown,
    };
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
