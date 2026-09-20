pub fn volume(input: &mut (f32, f32)) {
    let mut avg = input.0 + input.1;
    avg = avg.abs();
    let width = (avg * 40_f32) as usize;
    defmt::println!("{}", width)
}
