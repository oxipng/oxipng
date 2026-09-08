//! Time vs size across `-o` presets, the tradeoff the levels are about.
//! Run with `cargo bench --bench tradeoff`.

use std::time::Instant;

use malevich::{Align, Bars, Frame, Line, Plot, PointStyle, Points, Text};
use oxipng::{Options, optimize_from_memory};

fn main() {
    let png = std::fs::read("tests/files/rgb_16_should_be_grayscale_8.png")
        .expect("run from the crate root");
    let original = png.len() as f64;

    let mut labels = Vec::new();
    let mut ms = Vec::new();
    let mut sizes = Vec::new();
    println!("original: {} bytes", png.len());
    for level in 0..=6u8 {
        let opts = Options::from_preset(level);
        let start = Instant::now();
        let out = optimize_from_memory(&png, &opts).expect("optimize");
        let elapsed = start.elapsed().as_secs_f64() * 1_000.0;
        println!("-o{level}: {elapsed:7.1} ms  {} bytes", out.len());
        labels.push(level.to_string());
        ms.push(elapsed);
        sizes.push(out.len() as f64);
    }

    let saved: Vec<f64> = sizes.iter().map(|s| (1.0 - s / original) * 100.0).collect();

    let mut frame = Frame::detect();
    frame.width = frame.width.min(52);
    frame.height = 14;
    println!();
    print_path(&ms, &saved, &labels, &frame);
    frame.height = 12;
    print_bars("-o time, ms", &labels, &ms, &frame);
    print_bars("-o saved, %", &labels, &saved, &frame);
}

fn print_path(ms: &[f64], saved: &[f64], labels: &[String], frame: &Frame) {
    let xmax = ms.iter().copied().fold(0.0, f64::max) * 1.15;
    let ymin = saved.iter().copied().fold(f64::INFINITY, f64::min);
    let ymax = saved.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let pad = (ymax - ymin).max(1.0) * 0.2;
    let mut plot = Plot::new()
        .layer(Line::xy(ms, saved))
        .layer(Points::xy(ms, saved).style(PointStyle::Circle))
        .title("saved % vs time")
        .x_label("ms")
        .x_domain(0.0, xmax)
        .y_domain(ymin - pad, ymax + pad);
    for (i, label) in labels.iter().enumerate() {
        plot = plot.layer(Text::at(ms[i], saved[i], label.clone()).align(Align::Left));
    }
    println!("{}", plot.render_best(frame));
}

fn print_bars(title: &str, labels: &[String], values: &[f64], frame: &Frame) {
    println!(
        "{}",
        Plot::new()
            .layer(Bars::new(labels.iter().cloned(), values).color_by(labels.iter().cloned()),)
            .title(title)
            .render_best(frame)
    );
}
