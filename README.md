# esp32-s3 guitar pedal

> This is a guitar pedal that is coded in rust and can connect to wifi which is controlable by your phone and uses usb-c

## How it workes

- The guitar pedal uese a esp32-s3 communicating with a CS4270 CODEC throught I<sup>2</sup>C and I<sup>2</sup>S
- The esp32-s3 can be flash via usb and can then be controled by connecting to it via wifi and using a website
- The guitar pedal can use custom effects which can be coded in rust and used in the project

## Configuration

- Look at config.rs to change pin assignments if you want to make your own version of the pcb
- Add and remove audio modules in main.rs

## How to flash

1. Clone the Github reposotory using `git clone https://github.com/TMANTsmith/esp32-guitar-pedal/tree/main.git`
2. Install rust if you have not already using `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
3. Install the required packages
   '''shell
   cargo install ldproxy
   cargo install espflash --locked
   cargo install cargo-espflash --locked
   cargo install espup
   espup install
   '''
   asdf
4. Add the appropreate rust target `rustup target add xtensa-esp32s3-none-elf`
5. Build the firmware with `cargo build --release`
6. Plug in your esp32 using a usb-c cable and use `cargo espflash flash --release`
7. (Optional) use `cargo espflash flash --release` to monitor the progress
