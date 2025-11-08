use plotters::coord::Shift;
use plotters::prelude::*;
use std::f64::consts::PI;

// BitCrusher struct
struct BitCrush {
    value: f64, // Bit depth
}

impl BitCrush {
    fn new(value: f64) -> Self {
        match value {
            2.0 | 4.0 | 8.0 | 12.0 | 16.0 | 20.0 => (),
            _ => panic!("BitCrush value must be 2,4,8,12,16,20"),
        }
        BitCrush { value }
    }

    fn process(&self, input: &mut (f64, f64)) {
        let levels = 2f64.powf(self.value) - 1.0;
        // Map -1..1 -> 0..1, round, map back
        input.0 = ((input.0 + 1.0) / 2.0 * levels).round() / levels * 2.0 - 1.0;
        input.1 = ((input.1 + 1.0) / 2.0 * levels).round() / levels * 2.0 - 1.0;
    }

    fn process_list(&self, input: &mut [(f64, f64)]) {
        for tuple in input.iter_mut() {
            self.process(tuple);
        }
    }
}

// Generate sine wave
fn sine_wave(len: usize, freq: f64, sample_rate: f64) -> Vec<(f64, f64)> {
    (0..len)
        .map(|i| {
            let t = i as f64 / sample_rate;
            let s = (2.0 * PI * freq * t).sin(); // ±1 amplitude
            (s, s)
        })
        .collect()
}

// Draw waveform in a sub-area
fn draw_wave(
    drawing_area: &DrawingArea<BitMapBackend, Shift>,
    original: &[(f64, f64)],
    modified: &[(f64, f64)],
    title: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let len = original.len();
    let mut chart = ChartBuilder::on(drawing_area)
        .margin(5)
        .caption(title, ("sans-serif", 15))
        .x_label_area_size(20)
        .y_label_area_size(30)
        .build_cartesian_2d(0..len as i32, -1.2..1.2)?;

    chart.configure_mesh().disable_mesh().draw()?;

    // Original = Blue
    chart.draw_series(LineSeries::new(
        original.iter().enumerate().map(|(x, &(l, _))| (x as i32, l)),
        &BLUE,
    ))?;

    // Modified = Red
    chart.draw_series(LineSeries::new(
        modified.iter().enumerate().map(|(x, &(l, _))| (x as i32, l)),
        &RED,
    ))?;

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let sample_rate = 200.0; // higher sample count for smooth blue wave
    let len = 200;
    let freq = 5.0; // moderate frequency
    let original_wave = sine_wave(len, freq, sample_rate);

    let bit_depths = [2.0, 4.0, 8.0, 12.0, 16.0, 20.0];
    let total_cols = bit_depths.len();

    std::fs::create_dir_all("output")?;
    let root = BitMapBackend::new("output/bitcrush_grid.png", (1800, 400)).into_drawing_area();
    root.fill(&WHITE)?;
    let child_areas = root.split_evenly((1, total_cols));

    for (col, &depth) in bit_depths.iter().enumerate() {
        // Clone original so we do NOT modify it
        let mut modified_wave = original_wave.clone();
        let bitcrusher = BitCrush::new(depth);
        bitcrusher.process_list(&mut modified_wave);

        let title = format!("{} bits", depth);
        draw_wave(&child_areas[col], &original_wave, &modified_wave, &title)?;
    }

    println!("✅ PNG generated: output/bitcrush_grid.png");
    Ok(())
}

