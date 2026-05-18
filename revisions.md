# mini-pcb versions

## 1.0

### Features:

- Taking in audio from a guitar, giving it to the daisy seed, then outputting it to an amp
- Using 2 pads for a guitar pedal to act as a switch

### The Good

The output section of the audio worked well and could play a 440 Hz sine wave easily.

### The Bad

#### No Input Dectedted

The Problem: No signal was being passed to the daisy seed from a guitar.

The Explenation:

![Picture of the broken circuit](/pictures/broken_input.png)

The expected voltage on the node should be 2.5V but instead it is 3V. This causes the bias to be at 3V instead of 2.5V causing the daisy seed, which is biased at 2.5V, to read the wave improperly.

This is caused by R101 1M. This pulls the voltage up to 2.5V but because there is nothing going to GND then if the voltage goes above 2.5V there is nothing stopping it. But when shorting R101 the problem is fixed but this would mean the input impedance is 0 causing the guitar signal to be bad. The 1M being such a high value only lets a small amount of current through so if there is any amount of excess current coming from somewhere, like the capacitor, then it couldn't be dealt with

The Solution:

![Picture of the fixed circuit](/pictures/fixed_input.png)

1. Replace the capacitor with a C0G one instead of X5R to handle high impedance signals better.
2. Remove the LDO and use a resistor divider as the input impedance and bias.

#### Beeping

The Problem:

There was seemingly random fluctuating beeping when testing the daisy seed and when flashing. When the debug header was unplugged, there was a constant tone.

The Explanation:

The println statements were printing every time a pair of points was passed through causing there to be noise from the USB crossing over onto the audio line.

The Solution:

Remove excessive print statements

### Summary

Overall this is a success with getting the output of the circuit working, all I have to fix now is the input.
