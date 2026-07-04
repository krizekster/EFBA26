use drm::{Baseline, HeavyAbusive, HeavyReasonable, InGen, ProtectionProfile};
use macroquad::prelude::*;
use sim::Simulation;
use std::collections::VecDeque;

const W: f32 = 1280.0;
const H: f32 = 720.0;
const BOID_COUNT: usize = 5000;

enum Mode {
    Baseline,
    InGen,
    HeavyReasonable,
    HeavyAbusive,
}

impl Mode {
    fn name(&self) -> &'static str {
        match self {
            Mode::Baseline => "Baseline (No DRM)",
            Mode::InGen => "InGen (Lightweight)",
            Mode::HeavyReasonable => "Heavy-Reasonable (Sync VM/Thunks)",
            Mode::HeavyAbusive => "Heavy-Abusive (Worst Case)",
        }
    }
}

fn create_profile(mode: &Mode) -> Box<dyn ProtectionProfile> {
    match mode {
        Mode::Baseline => Box::new(Baseline),
        Mode::InGen => Box::new(InGen::new()),
        Mode::HeavyReasonable => Box::new(HeavyReasonable::new()),
        Mode::HeavyAbusive => Box::new(HeavyAbusive::new()),
    }
}

fn window_conf() -> Conf {
    Conf {
        window_title: "DRM Benchmark Testbed".to_owned(),
        window_width: W as i32,
        window_height: H as i32,
        high_dpi: true,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut sim = Simulation::new(BOID_COUNT, W, H, 12345);
    let mut mode = Mode::Baseline;
    let mut profile = create_profile(&mode);
    profile.on_startup();
    let _asset = profile.load_protected_asset().unwrap_or_default();

    let mut frame_times: VecDeque<f32> = VecDeque::with_capacity(300);

    loop {
        let mut mode_changed = false;
        if is_key_pressed(KeyCode::Key1) { mode = Mode::Baseline; mode_changed = true; }
        if is_key_pressed(KeyCode::Key2) { mode = Mode::InGen; mode_changed = true; }
        if is_key_pressed(KeyCode::Key3) { mode = Mode::HeavyReasonable; mode_changed = true; }
        if is_key_pressed(KeyCode::Key4) { mode = Mode::HeavyAbusive; mode_changed = true; }

        if mode_changed {
            sim = Simulation::new(BOID_COUNT, W, H, 12345);
            profile = create_profile(&mode);
            let start = get_time();
            profile.on_startup();
            let _asset = profile.load_protected_asset().unwrap_or_default();
            println!("Switched to {} in {:.2}ms", mode.name(), (get_time() - start) * 1000.0);
            frame_times.clear();
        }

        let sim_start = get_time();
        sim.step(1.0 / 60.0);
        profile.on_checkpoint();
        let sim_time = (get_time() - sim_start) as f32 * 1000.0;
        
        if frame_times.len() >= 300 {
            frame_times.pop_front();
        }
        frame_times.push_back(sim_time);

        clear_background(Color::new(0.05, 0.05, 0.05, 1.0));

        for boid in &sim.boids {
            let dir = boid.velocity.normalize_or_zero();
            let p1 = macroquad::math::vec2(boid.position.x + dir.x * 6.0, boid.position.y + dir.y * 6.0);
            let p2 = macroquad::math::vec2(boid.position.x - dir.x * 3.0 + dir.y * 2.0, boid.position.y - dir.y * 3.0 - dir.x * 2.0);
            let p3 = macroquad::math::vec2(boid.position.x - dir.x * 3.0 - dir.y * 2.0, boid.position.y - dir.y * 3.0 + dir.x * 2.0);
            draw_triangle(p1, p2, p3, Color::new(0.4, 0.8, 1.0, 0.8));
        }

        let mut sorted: Vec<f32> = frame_times.iter().copied().collect();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        
        let avg = if sorted.is_empty() { 0.0 } else { sorted.iter().sum::<f32>() / sorted.len() as f32 };
        let one_percent_count = ((sorted.len() as f32 * 0.01).ceil() as usize).max(1);
        let slowest_1 = &sorted[sorted.len().saturating_sub(one_percent_count)..];
        let low1 = if slowest_1.is_empty() { 0.0 } else { slowest_1.iter().sum::<f32>() / slowest_1.len() as f32 };
        
        let p01_count = ((sorted.len() as f32 * 0.001).ceil() as usize).max(1);
        let slowest_01 = &sorted[sorted.len().saturating_sub(p01_count)..];
        let low01 = if slowest_01.is_empty() { 0.0 } else { slowest_01.iter().sum::<f32>() / slowest_01.len() as f32 };
        
        let fps = get_fps();

        draw_rectangle(10.0, 10.0, 420.0, 190.0, Color::new(0.0, 0.0, 0.0, 0.7));
        draw_text("DRM Benchmark Testbed", 20.0, 40.0, 30.0, WHITE);
        draw_text(&format!("Current Mode: {}", mode.name()), 20.0, 70.0, 20.0, YELLOW);
        draw_text(&format!("FPS: {}", fps), 20.0, 100.0, 20.0, WHITE);
        draw_text(&format!("Sim Time: {:.2} ms", sim_time), 20.0, 120.0, 20.0, WHITE);
        draw_text(&format!("Avg Sim Time: {:.2} ms", avg), 20.0, 140.0, 20.0, WHITE);
        draw_text(&format!("1% Low Sim Time: {:.2} ms", low1), 20.0, 160.0, 20.0, RED);
        draw_text(&format!("0.1% Low Sim Time: {:.2} ms", low01), 20.0, 180.0, 20.0, RED);

        draw_text("Keys: 1=Baseline, 2=InGen, 3=HeavyReasonable, 4=HeavyAbusive", 20.0, H - 20.0, 20.0, GRAY);

        next_frame().await;
    }
}
