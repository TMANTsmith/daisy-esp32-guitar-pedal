use settings::consts::*;

fn main() {
    let js = format!(
        r#"window.RUST_CONSTS = {{
  FFT_BINS: {fft_bins},
  SAMPLE_RATE: {sample_rate},
  INPUT_IS_DB: {input_is_db},
  DB_MIN: {db_min},
  DB_MAX: {db_max},
  MIN_DISPLAY_FREQ: {min_display_freq},
  SMOOTHING: {smoothing},
  PEAK_DECAY_DB_PER_SEC: {peak_decay},
  PEAK_MIN_DB: {peak_min_db},
  PEAK_COUNT: {peak_count},
  SPECTRUM_SIZE: {spectrum_size},
  HZ_PER_BIN: {hz_per_bin},
  MAX_DISPLAY_FREQ: {max_display_freq},
  PEAK_MIN_SEPARATION_BINS: {peak_min_sep},
}};
"#,
        fft_bins = FFT_BINS,
        sample_rate = SAMPLE_RATE,
        input_is_db = INPUT_IS_DB,
        db_min = DB_MIN,
        db_max = DB_MAX,
        min_display_freq = MIN_DISPLAY_FREQ,
        smoothing = SMOOTHING,
        peak_decay = PEAK_DECAY_DB_PER_SEC,
        peak_min_db = PEAK_MIN_DB,
        peak_count = PEAK_COUNT,
        spectrum_size = spectrum_size(),
        hz_per_bin = hz_per_bin(),
        max_display_freq = max_display_freq(),
        peak_min_sep = peak_min_separation_bins(),
    );

    let out_dir = std::env::var("OUT_DIR").unwrap();
    std::fs::write(std::path::Path::new(&out_dir).join("consts.js"), &js)
        .expect("failed to write consts.js to OUT_DIR");

    let static_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/consts.js");
    std::fs::write(&static_path, &js)
        .expect("failed to write consts.js to src dir");

    println!("cargo:rerun-if-changed=../settings/src/consts.rs");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rustc-link-arg=-Tdefmt.x");
    println!("cargo:rustc-link-arg=-Tlinkall.x");
}

fn hz_per_bin() -> f64 {
    SAMPLE_RATE as f64 / 2.0 / FFT_BINS as f64
}
fn spectrum_size() -> usize {
    (20000.0 / hz_per_bin()).ceil() as usize
}
fn max_display_freq() -> f64 {
    spectrum_size() as f64 * hz_per_bin()
}
fn peak_min_separation_bins() -> usize {
    4usize.max((spectrum_size() as f64 / 40.0).round() as usize)
}
