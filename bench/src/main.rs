use drm::{Baseline, HeavyAbusive, HeavyReasonable, InGen, AlwaysOnline, VMAntitamper, ProtectionProfile};
use sim::Simulation;
use std::fs::File;
use std::io::Write;
use std::time::Instant;

fn run_benchmark(mode_name: &str, mut profile: Box<dyn ProtectionProfile>) -> Vec<f64> {
    println!("Benchmarking {}...", mode_name);
    let mut sim = Simulation::new(5000, 800.0, 600.0, 12345);
    let dt = 1.0 / 60.0;
    
    let startup_start = Instant::now();
    profile.on_startup();
    let startup_time = startup_start.elapsed().as_secs_f64();
    println!("  Startup time: {:.4}s", startup_time);
    
    for _ in 0..10 {
        sim.step(dt);
        profile.on_checkpoint();
    }
    
    let frames = 200;
    let mut durations = Vec::with_capacity(frames);
    
    for _ in 0..frames {
        let start = Instant::now();
        sim.step(dt);
        profile.on_checkpoint();
        durations.push(start.elapsed().as_secs_f64() * 1000.0);
    }
    
    durations
}

fn calculate_stats(durations: &[f64]) -> (f64, f64, f64, f64, f64) {
    let mut sorted = durations.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    
    let avg = sorted.iter().sum::<f64>() / sorted.len() as f64;
    let p95 = sorted[(sorted.len() as f64 * 0.95) as usize];
    let p99 = sorted[(sorted.len() as f64 * 0.99) as usize];
    
    let one_percent_count = (sorted.len() as f64 * 0.01).ceil() as usize;
    let slowest_1_percent = &sorted[sorted.len() - one_percent_count..];
    let one_percent_low = slowest_1_percent.iter().sum::<f64>() / one_percent_count as f64;
    
    let p01_count = (sorted.len() as f64 * 0.001).ceil() as usize;
    let slowest_01_percent = &sorted[sorted.len() - p01_count..];
    let p01_low = slowest_01_percent.iter().sum::<f64>() / p01_count as f64;
    
    (avg, p95, p99, one_percent_low, p01_low)
}

fn main() {
    let mut csv = File::create("benchmark_results.csv").unwrap();
    writeln!(csv, "Mode,Avg (ms),P95 (ms),P99 (ms),1% Low (ms),0.1% Low (ms)").unwrap();
    
    let modes: Vec<(&str, Box<dyn ProtectionProfile>)> = vec![
        ("Baseline", Box::new(Baseline)),
        ("InGen", Box::new(InGen::new())),
        ("Heavy-Reasonable", Box::new(HeavyReasonable::new())),
        ("Heavy-Abusive", Box::new(HeavyAbusive::new())),
        ("Always-Online", Box::new(AlwaysOnline::new())),
        ("VM-Antitamper", Box::new(VMAntitamper::new())),
    ];
    
    for (name, profile) in modes {
        let durations = run_benchmark(name, profile);
        let (avg, p95, p99, one_percent_low, p01_low) = calculate_stats(&durations);
        
        println!("  Avg: {:.2}ms", avg);
        println!("  P95: {:.2}ms", p95);
        println!("  P99: {:.2}ms", p99);
        println!("  1% Low: {:.2}ms", one_percent_low);
        println!("  0.1% Low: {:.2}ms\n", p01_low);
        
        writeln!(csv, "{},{:.2},{:.2},{:.2},{:.2},{:.2}", name, avg, p95, p99, one_percent_low, p01_low).unwrap();
    }
    
    println!("Wrote results to benchmark_results.csv");
}
