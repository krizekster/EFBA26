use drm::{Baseline, HeavyAbusive, HeavyReasonable, InGen, AlwaysOnline, VMAntitamper, ProtectionProfile, TAMPER_FLAG};
use macroquad::prelude::*;
use sim::Simulation;
use std::collections::VecDeque;
use std::sync::atomic::Ordering;
use std::fs;

const W: f32 = 1280.0;
const H: f32 = 720.0;
const BOID_COUNT: usize = 5000;

enum Mode {
    Baseline,
    InGen,
    HeavyReasonable,
    HeavyAbusive,
    AlwaysOnline,
    VMAntitamper,
}

#[derive(PartialEq)]
enum VisualTheme {
    Cubes,
    VoxelTerrain,
    WireframeSpace,
}

impl VisualTheme {
    fn name(&self) -> &'static str {
        match self {
            VisualTheme::Cubes => "Abstract Cubes",
            VisualTheme::VoxelTerrain => "Voxel Terrain",
            VisualTheme::WireframeSpace => "Wireframe Space",
        }
    }
}

impl Mode {
    fn name(&self) -> &'static str {
        match self {
            Mode::Baseline => "Baseline (No DRM)",
            Mode::InGen => "InGen (Lightweight)",
            Mode::HeavyReasonable => "Heavy-Reasonable (Sync VM/Thunks)",
            Mode::HeavyAbusive => "Heavy-Abusive (Worst Case)",
            Mode::AlwaysOnline => "Always Online (Network micro-stutters)",
            Mode::VMAntitamper => "VM Anti-tamper (Denuvo-style obfuscation)",
        }
    }
}

fn create_profile(mode: &Mode) -> Box<dyn ProtectionProfile> {
    match mode {
        Mode::Baseline => Box::new(Baseline),
        Mode::InGen => Box::new(InGen::new()),
        Mode::HeavyReasonable => Box::new(HeavyReasonable::new()),
        Mode::HeavyAbusive => Box::new(HeavyAbusive::new()),
        Mode::AlwaysOnline => Box::new(AlwaysOnline::new()),
        Mode::VMAntitamper => Box::new(VMAntitamper::new()),
    }
}

fn window_conf() -> Conf {
    Conf {
        window_title: "3D Pixel Game - DRM Testbed".to_owned(),
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
    let mut theme = VisualTheme::Cubes;

    // Crack simulation flags
    let mut token_crack_attempted = false;
    let mut tampered = false;

    loop {
        // Handle input for Mode switching
        let mut mode_changed = false;
        if is_key_pressed(KeyCode::Key1) { mode = Mode::Baseline; mode_changed = true; }
        if is_key_pressed(KeyCode::Key2) { mode = Mode::InGen; mode_changed = true; }
        if is_key_pressed(KeyCode::Key3) { mode = Mode::HeavyReasonable; mode_changed = true; }
        if is_key_pressed(KeyCode::Key4) { mode = Mode::HeavyAbusive; mode_changed = true; }
        if is_key_pressed(KeyCode::Key5) { mode = Mode::AlwaysOnline; mode_changed = true; }
        if is_key_pressed(KeyCode::Key6) { mode = Mode::VMAntitamper; mode_changed = true; }

        if is_key_pressed(KeyCode::M) {
            theme = match theme {
                VisualTheme::Cubes => VisualTheme::VoxelTerrain,
                VisualTheme::VoxelTerrain => VisualTheme::WireframeSpace,
                VisualTheme::WireframeSpace => VisualTheme::Cubes,
            };
        }

        if mode_changed {
            sim = Simulation::new(BOID_COUNT, W, H, 12345);
            TAMPER_FLAG.store(false, Ordering::Relaxed);
            token_crack_attempted = false;
            tampered = false;
            
            profile = create_profile(&mode);
            let start = get_time();
            profile.on_startup();
            let _asset = profile.load_protected_asset().unwrap_or_default();
            println!("Switched to {} in {:.2}ms", mode.name(), (get_time() - start) * 1000.0);
            frame_times.clear();
        }

        // Handle crack simulation
        if is_key_pressed(KeyCode::C) {
            println!("Simulating Memory Tamper (Cheat Engine)...");
            TAMPER_FLAG.store(true, Ordering::Relaxed);
        }
        
        if is_key_pressed(KeyCode::T) {
            println!("Simulating Invalid Token Injection...");
            token_crack_attempted = true;
            // In a real crack, this would mean the verify fails
        }

        tampered = TAMPER_FLAG.load(Ordering::Relaxed) || token_crack_attempted;

        let sim_start = get_time();
        if !tampered {
            sim.step(1.0 / 60.0);
            profile.on_checkpoint();
        }
        let sim_time = (get_time() - sim_start) as f32 * 1000.0;
        
        if frame_times.len() >= 300 {
            frame_times.pop_front();
        }
        frame_times.push_back(sim_time);

        clear_background(Color::new(0.05, 0.05, 0.1, 1.0));

        // Setup 3D Camera
        set_camera(&Camera3D {
            position: vec3(W / 2.0, 600.0, -300.0),
            up: vec3(0.0, 1.0, 0.0),
            target: vec3(W / 2.0, 0.0, H / 2.0),
            ..Default::default()
        });

        // Draw 3D Entities based on Theme
        for (i, boid) in sim.boids.iter().enumerate() {
            let color = if i % 3 == 0 { RED } else if i % 3 == 1 { GREEN } else { BLUE };
            let size = 6.0;
            let hover_y = (get_time() as f32 * 2.0 + (i as f32)).sin() * 10.0;
            
            match theme {
                VisualTheme::Cubes => {
                    draw_cube(vec3(boid.position.x, hover_y, boid.position.y), vec3(size, size, size), None, color);
                }
                VisualTheme::VoxelTerrain => {
                    // Draw a grid-like voxel pillar
                    let base_y = -20.0;
                    draw_cube(vec3(boid.position.x, base_y + hover_y, boid.position.y), vec3(size*2.0, size*4.0, size*2.0), None, DARKGREEN);
                }
                VisualTheme::WireframeSpace => {
                    // Draw wireframe asteriods/ships (using lines)
                    let p = vec3(boid.position.x, hover_y * 3.0, boid.position.y);
                    draw_line_3d(p + vec3(-size, 0.0, -size), p + vec3(size, 0.0, -size), WHITE);
                    draw_line_3d(p + vec3(size, 0.0, -size), p + vec3(0.0, 0.0, size), WHITE);
                    draw_line_3d(p + vec3(0.0, 0.0, size), p + vec3(-size, 0.0, -size), WHITE);
                }
            }
        }

        // Back to 2D for UI
        set_default_camera();

        // UI Performance Overlay
        let mut sorted: Vec<f32> = frame_times.iter().copied().collect();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        
        let avg = if sorted.is_empty() { 0.0 } else { sorted.iter().sum::<f32>() / sorted.len() as f32 };
        let one_percent_count = ((sorted.len() as f32 * 0.01).ceil() as usize).max(1);
        let slowest_1 = &sorted[sorted.len().saturating_sub(one_percent_count)..];
        let low1 = if slowest_1.is_empty() { 0.0 } else { slowest_1.iter().sum::<f32>() / slowest_1.len() as f32 };
        
        let fps = get_fps();

        draw_rectangle(10.0, 10.0, 420.0, 240.0, Color::new(0.0, 0.0, 0.0, 0.7));
        draw_text("3D DRM Testbed & Security", 20.0, 40.0, 30.0, WHITE);
        draw_text(&format!("Current Mode: {}", mode.name()), 20.0, 70.0, 20.0, YELLOW);
        draw_text(&format!("Visual Theme: {} (Press M)", theme.name()), 20.0, 95.0, 20.0, GREEN);
        draw_text(&format!("FPS: {}", fps), 20.0, 120.0, 20.0, WHITE);
        draw_text(&format!("Sim Time: {:.2} ms", sim_time), 20.0, 140.0, 20.0, WHITE);
        draw_text(&format!("Avg Sim Time: {:.2} ms", avg), 20.0, 160.0, 20.0, WHITE);
        draw_text(&format!("1% Low Sim Time: {:.2} ms", low1), 20.0, 180.0, 20.0, RED);

        draw_text("DRM Control: 1=Base, 2=InGen, 3=HeavyR, 4=HeavyA, 5=Online, 6=VMAntitamper", 20.0, 210.0, 16.0, GRAY);
        
        draw_text("Crack Simulator:", 20.0, 230.0, 16.0, ORANGE);
        draw_text("Press 'C' -> Memory Tamper (Cheat Engine)", 20.0, 245.0, 16.0, ORANGE);
        draw_text("Press 'T' -> Token Bypass (Invalid License)", 20.0, 260.0, 16.0, ORANGE);

        // --- Live Telemetry Panel ---
        draw_rectangle(W - 600.0, 10.0, 590.0, 180.0, Color::new(0.0, 0.1, 0.0, 0.8));
        draw_text("LIVE DRM TELEMETRY / SECURITY LOGS", W - 580.0, 35.0, 20.0, GREEN);
        
        if let Ok(log_content) = fs::read_to_string("security_audit.log") {
            let lines: Vec<&str> = log_content.lines().collect();
            let mut display_lines = Vec::new();
            for i in (0..lines.len()).rev().take(7) {
                display_lines.push(lines[i]);
            }
            display_lines.reverse();
            
            let mut y_offset = 60.0;
            for line in display_lines {
                let color = if line.contains("[CRITICAL]") { RED } else { LIGHTGRAY };
                draw_text(line, W - 580.0, y_offset, 14.0, color);
                y_offset += 16.0;
            }
        }

        // Tamper Overlay
        if tampered {
            draw_rectangle(0.0, 0.0, W, H, Color::new(1.0, 0.0, 0.0, 0.4));
            let msg = if token_crack_attempted {
                "DRM VIOLATION: INVALID LICENSE TOKEN SIGNATURE!"
            } else {
                "DRM VIOLATION: MEMORY TAMPERING DETECTED!"
            };
            let text_size = measure_text(msg, None, 40, 1.0);
            draw_text(
                msg,
                W / 2.0 - text_size.width / 2.0,
                H / 2.0,
                40.0,
                WHITE,
            );
            draw_text(
                "Gameplay Halted. Press '1' to reset.",
                W / 2.0 - 200.0,
                H / 2.0 + 40.0,
                30.0,
                WHITE,
            );
        }

        next_frame().await;
    }
}
