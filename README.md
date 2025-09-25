# esp32-s3 guitar pedal

### This is a guitar pedal that is coded in rust and can connect to wifi

## How it workes

- The guitar pedal uese a esp32-s3 communicating with a CS4270 CODEC throught I<sup>2<sup/>C
- The esp32-s3 can but flash via usb
- The pedal uses a buck-converter to convert the 3.7 Li-Po battery voltage into stable 3.3v
