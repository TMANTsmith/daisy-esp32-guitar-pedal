use plotters::coord::Shift;
use plotters::prelude::*;
use std::f64::consts::PI;

// Fixed Fuzz struct with only fuzz and bais
struct Fuzz {
    fuzz: f64,
    bais: f64,
}

impl Fuzz {
    fn new(fuzz: f64, bais: f64) -> Self {
        Fuzz { fuzz, bais }
    }

    fn process(&self, input: &mut (f64, f64)) {
        input.0 = ((input.0 + self.bais) * (self.fuzz + 1.0) * (self.fuzz + 1.0)).tanh();
        input.1 = ((input.1 + self.bais) * (self.fuzz + 1.0) * (self.fuzz + 1.0)).tanh();
    }

    fn process_list(&self, input: &mut [(f64, f64)]) {
        for tuple in input.iter_mut() {
            self.process(tuple);
        }
    }
}

// Generate high-frequency sine wave
fn high_freq_wave(len: usize, freq: f64, sample_rate: f64) -> Vec<(f64, f64)> {
    (0..len)
        .map(|i| {
            let t = i as f64 / sample_rate;
            let s = (2.0 * PI * freq * t).sin();
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

    chart.draw_series(LineSeries::new(
        original.iter().enumerate().map(|(x, &(l, _))| (x as i32, l)),
        &BLUE,
    ))?;

    chart.draw_series(LineSeries::new(
        modified.iter().enumerate().map(|(x, &(l, _))| (x as i32, l)),
        &RED,
    ))?;

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let sample_rate = 1000.0;
    let len = 200;
    let freq = 20.0; // high-frequency wave
    let original_wave = high_freq_wave(len, freq, sample_rate);

    let values = [0.0, 0.25, 0.5, 0.75, 1.0]; // fuzz variations

    let total_rows = 1;
    let total_cols = values.len();

    let root = BitMapBackend::new("fuzz_only.png", (1200, 300)).into_drawing_area();
    root.fill(&WHITE)?;
    let child_areas = root.split_evenly((total_rows, total_cols));

    for (col, &v) in values.iter().enumerate() {
        let mut modified_wave = original_wave.clone();
        let fuzz_struct = Fuzz::new(v, 0.0); // bais neutral
        fuzz_struct.process_list(&mut modified_wave);

        let title = format!("Fuzz={}", v);
        draw_wave(&child_areas[col], &original_wave, &modified_wave, &title)?;
    }

    println!("PNG generated as fuzz_only.png");
    Ok(())
}

