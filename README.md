# Daisy Esp32 Pedal

This is a guitar pedal with the goal of combining the audio processing power of the Daisy Seed with the wifi capabilities of the Esp32C series (currently only esting the C6). This way you can fully control the guitar pedal with your phone connected by a website hosted by the esp32 and for low-latantcy tasks physical buttons will be on the board. This will allow for greator flexability through the user being able to add or remove buttons by making changes to the website allowing them to have any configuration they want.

## What is used

### Hardware

The mini-pcb which includes TRS jacks for audio processing does not currently have the esp32 but processing audio with just the daisy seed is working

The full-pcb inclued a esp32 which is connected to the daisy seed through SPI and includes 2 TS jacks this pcb has ben designed but not tested yet

### Software

The project is written in rust and uses embassy as the async exacutor. Specifily the [daisy-embassy](https://github.com/daisy-embassy/daisy-embassy/tree/master) which has some audio stuff, like a DMA buffer, built in.

## Examples

So far there is only one example which is just on main.rs which does a FFT using [microfft](https://gitlab.com/teskje/microfft-rs) but includes things like using the SDRAM on the seed as the heap, interupts and how to share information between them, how to make and use buffer for audio anaysis, and more.
