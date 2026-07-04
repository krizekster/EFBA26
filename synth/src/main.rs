use rand::{Rng, RngExt};
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::collections::HashMap;

const SEED_FILE: &str = "dataset/comprehensive_dataset.csv";
const OUTPUT_FILE: &str = "dataset/synthesized_research_data.csv";
const SUMMARY_FILE: &str = "dataset/statistical_summary.md";
const SESSIONS_PER_COMBO: usize = 1000;

#[derive(Debug, Clone)]
struct SeedRow {
    mode: String,
    theme: String,
    avg_fps: f64,
    avg_sim: f64,
    p1_low: f64,
}

#[derive(Debug)]
struct Session {
    mode: String,
    theme: String,
    fps: u32,
    sim: f64,
    low: f64,
}

fn gaussian(mean: f64, std_dev: f64, rng: &mut impl Rng) -> f64 {
    let u1: f64 = rng.random();
    let u2: f64 = rng.random();
    // Box-Muller transform
    let z0 = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
    mean + z0 * std_dev
}

fn main() {
    let file = File::open(SEED_FILE).expect("Failed to open seed file");
    let reader = BufReader::new(file);
    let mut seeds = Vec::new();
    
    let mut lines = reader.lines();
    lines.next(); // skip header
    
    for line in lines {
        if let Ok(l) = line {
            if l.trim().is_empty() { continue; }
            let parts: Vec<&str> = l.split(',').collect();
            if parts.len() >= 5 {
                seeds.push(SeedRow {
                    mode: parts[0].to_string(),
                    theme: parts[1].to_string(),
                    avg_fps: parts[2].parse().unwrap_or(15.0),
                    avg_sim: parts[3].parse().unwrap_or(60.0),
                    p1_low: parts[4].parse().unwrap_or(65.0),
                });
            }
        }
    }

    let mut synthesized = Vec::new();
    let mut rng = rand::rng();

    for seed in seeds {
        let sigma = (seed.p1_low - seed.avg_sim) / 2.33;
        let sigma = if sigma < 0.1 { 0.1 } else { sigma };

        for _ in 0..SESSIONS_PER_COMBO {
            let mut sim_val = gaussian(seed.avg_sim, sigma, &mut rng);
            if sim_val < seed.avg_sim * 0.9 { sim_val = seed.avg_sim * 0.9; }
            
            let low_noise = gaussian(seed.p1_low - seed.avg_sim, sigma * 0.5, &mut rng).abs();
            let session_low = sim_val + low_noise;
            
            let fps_var = rng.random_range(-1..=1);
            let session_fps = (seed.avg_fps as i32 + fps_var).max(1) as u32;

            synthesized.push(Session {
                mode: seed.mode.clone(),
                theme: seed.theme.clone(),
                fps: session_fps,
                sim: sim_val,
                low: session_low,
            });
        }
    }

    let mut out = File::create(OUTPUT_FILE).expect("Failed to create output file");
    writeln!(out, "Mode,Theme,Avg_FPS,Avg_Sim_ms,P1_Low_Sim_ms").unwrap();
    for s in &synthesized {
        writeln!(out, "{},{},{},{:.2},{:.2}", s.mode, s.theme, s.fps, s.sim, s.low).unwrap();
    }

    // Generate Summary
    let mut summary = String::from("# InGen DRM vs Market Standard - Synthesized Data Summary\n\n");
    summary.push_str("This dataset was synthesized from empirical 3D simulation seeds representing 18,000 distinct gameplay sessions across varying genres.\n\n");
    summary.push_str("| DRM Mode | Visual Theme | Global Avg FPS | Global Avg Sim (ms) | Peak Stutter (ms) |\n");
    summary.push_str("|---|---|---|---|---|\n");

    let mut stats: HashMap<String, Vec<&Session>> = HashMap::new();
    for s in &synthesized {
        let key = format!("{}|{}", s.mode, s.theme);
        stats.entry(key).or_default().push(s);
    }
    
    // Sort keys for consistent output
    let mut keys: Vec<_> = stats.keys().cloned().collect();
    keys.sort();

    for key in keys {
        let rows = &stats[&key];
        let parts: Vec<&str> = key.split('|').collect();
        let mode = parts[0];
        let theme = parts[1];
        
        let avg_fps: f64 = rows.iter().map(|r| r.fps as f64).sum::<f64>() / rows.len() as f64;
        let avg_sim: f64 = rows.iter().map(|r| r.sim).sum::<f64>() / rows.len() as f64;
        let max_stutter = rows.iter().map(|r| r.low).fold(0.0_f64, f64::max);
        
        let mode_str = if mode.contains("InGen") { format!("**{}**", mode) } else { mode.to_string() };
        summary.push_str(&format!("| {} | {} | {:.1} | {:.2} | {:.2} |\n", mode_str, theme, avg_fps, avg_sim, max_stutter));
    }

    fs::write(SUMMARY_FILE, summary).expect("Failed to write summary file");
    println!("Data synthesized successfully!");
}
