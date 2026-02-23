# N64 Flashcart Communication Library

Provides the crate `n64flashcart`, which wraps [UNFLoader's](https://github.com/buu342/N64-UNFLoader/) flashcart library for Rust.

A small proof-of-concept client application is included as the `zootr-usb-testing` crate. It interfaces with [this fork of the Ocarina of Time randomizer](https://github.com/mracsys/OoT-Randomizer/tree/summercart_mw_support) to perform the following:

- Listen for a custom handshake heartbeat from the console, then complete the handshake per [Fenhl's spec](https://github.com/OoTRandomizer/OoT-Randomizer/issues/2042).
  - The handshake is initiated by the console sending the heartbeat instead of the PC client periodically sending `cmdt` to the console, but the process is otherwise identical.
- Wait until the player is in-game, detected by receiving a save context packet from console.
- Wait 10 seconds, then send a packet to console to give the player the Light Arrows item.
- Continue listening for messages until all of the following are received from console:
  - Acknowledgement packet for the Light Arrows receipt
  - Dungeon info packet by pressing the A button in the pause screen
  - Item sent packet for an item in another player's world (i.e. an item for Player 2 while the console is playing as Player 1, or vice versa)

## Building from source

No binaries are included in this repo. The program can either be run directly with `cargo run` in the root of the repo, or manually built with `cargo build` or `cargo build --release`.

### Windows

No additional requirements.

### Linux

Requires the following dependencies:

- libusb-1.0
- libftdi

Example for Fedora:
```bash
sudo dnf install libusb1-devel libftdi-devel
```

### macOS

Requires the following dependencies via Homebrew:

- libusb
- libftdi

```bash
brew install libusb libftdi
```
