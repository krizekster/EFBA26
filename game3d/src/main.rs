use drm::{Baseline, HeavyAbusive, HeavyReasonable, InGen, AlwaysOnline, VMAntitamper, ProtectionProfile, TAMPER_FLAG};
use macroquad::prelude::*;
use sim::Simulation;
use std::collections::VecDeque;
use std::sync::atomic::Ordering;
use std::fs::{self, OpenOptions};
use std::io::Write;

const W: f32 = 1280.0;
const H: f32 = 720.0;
const BOID_COUNT: usize = 5000;

#[derive(PartialEq, Clone, Copy)]
enum Mode {
    Baseline,
    InGen,
    HeavyReasonable,
    HeavyAbusive,
    AlwaysOnline,
    VMAntitamper,
}

#[derive(PartialEq, Clone, Copy)]
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
            Mode::VMAntitamper => "VM-Antitamper (Sync Hash Loop)",
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

fn draw_button(x: f32, y: f32, w: f32, h: f32, text: &str, active: bool, cursor_grabbed: bool) -> bool {
    let (mx, my) = mouse_position();
    let hovered = !cursor_grabbed && mx >= x && mx <= x + w && my >= y && my <= y + h;
    let color = if active { DARKGREEN } else if hovered { GRAY } else { DARKGRAY };
    
    draw_rectangle(x, y, w, h, color);
    draw_rectangle_lines(x, y, w, h, 2.0, LIGHTGRAY);
    
    let text_size = measure_text(text, None, 16, 1.0);
    draw_text(text, x + w/2.0 - text_size.width/2.0, y + h/2.0 + text_size.offset_y/2.0, 16.0, WHITE);
    
    hovered && is_mouse_button_pressed(MouseButton::Left)
}

fn log_session(mode_name: &str, avg: f32, low1: f32) {
    let _ = fs::create_dir_all("logs");
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open("logs/session_history.log") {
        let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
        let _ = writeln!(f, "[{}] Mode: {} | Avg: {:.2}ms | 1% Low: {:.2}ms", ts, mode_name, avg, low1);
    }
}

struct Projectile {
    pos: Vec3,
    dir: Vec3,
    active: bool,
}

fn window_conf() -> Conf {
    Conf {
        window_title: "InGen DRM 3D Testbed & Security FPS".to_owned(),
        window_width: W as i32,
        window_height: H as i32,
        high_dpi: true,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let auto_bench = std::env::args().any(|arg| arg == "--auto-benchmark");
    let mut auto_timer = 0.0;
    let auto_modes = [Mode::Baseline, Mode::InGen, Mode::HeavyReasonable, Mode::HeavyAbusive, Mode::AlwaysOnline, Mode::VMAntitamper];
    let auto_themes = [VisualTheme::Cubes, VisualTheme::VoxelTerrain, VisualTheme::WireframeSpace];
    let mut auto_mode_idx = 0;
    let mut auto_theme_idx = 0;
    let mut auto_dataset = None;

    let mut sim = Simulation::new(BOID_COUNT, W, H, 12345);
    let mut mode = Mode::Baseline;
    let mut profile = create_profile(&mode);
    
    let _ = profile.on_startup();
    let _asset = profile.load_protected_asset().unwrap_or_default();

    let mut frame_times: VecDeque<f32> = VecDeque::with_capacity(300);
    let mut theme = VisualTheme::Cubes;

    let mut token_crack_attempted = false;
    let mut tampered = false;

    // FPS Gameplay State
    let mut player_pos = vec3(W / 2.0, 20.0, -300.0);
    let mut pitch: f32 = 0.0;
    let mut yaw: f32 = std::f32::consts::PI / 2.0;
    let mut cursor_grabbed = false;
    let mut projectiles: Vec<Projectile> = Vec::new();
    let mut score = 0;
    let mut boid_active = vec![true; BOID_COUNT];

    // Initialize session logging
    let _ = fs::create_dir_all("logs");

    if auto_bench {
        println!("--- AUTOMATED BENCHMARK SEQUENCE STARTED ---");
        let _ = fs::create_dir_all("dataset");
        auto_dataset = Some(OpenOptions::new().create(true).write(true).truncate(true).open("dataset/comprehensive_dataset.csv").unwrap());
        let _ = writeln!(auto_dataset.as_mut().unwrap(), "Mode,Theme,Avg_FPS,Avg_Sim_ms,P1_Low_Sim_ms");
    }

    loop {
        let dt = get_frame_time();
        let mut mode_changed = false;

        // Auto Benchmark Logic
        if auto_bench {
            auto_timer += dt;
            // Draw a big overlay saying Auto Benchmarking
            draw_rectangle(0.0, 0.0, W, H, Color::new(0.0, 0.0, 0.0, 0.5));
            draw_text("AUTOMATED DATA COLLECTION IN PROGRESS...", W/2.0 - 300.0, H/2.0 - 50.0, 40.0, RED);
            draw_text(&format!("Testing: {} + {}", mode.name(), theme.name()), W/2.0 - 300.0, H/2.0 + 20.0, 30.0, WHITE);
            
            if auto_timer > 3.0 { // 3 seconds per combination
                let mut sorted: Vec<f32> = frame_times.iter().copied().collect();
                sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
                let avg = if sorted.is_empty() { 0.0 } else { sorted.iter().sum::<f32>() / sorted.len() as f32 };
                let one_percent_count = ((sorted.len() as f32 * 0.01).ceil() as usize).max(1);
                let slowest_1 = &sorted[sorted.len().saturating_sub(one_percent_count)..];
                let low1 = if slowest_1.is_empty() { 0.0 } else { slowest_1.iter().sum::<f32>() / slowest_1.len() as f32 };
                
                let _ = writeln!(auto_dataset.as_mut().unwrap(), "{},{},{},{:.2},{:.2}", mode.name(), theme.name(), get_fps(), avg, low1);
                println!("Collected data for {} + {}: FPS {}, Avg {:.2}ms, 1% Low {:.2}ms", mode.name(), theme.name(), get_fps(), avg, low1);

                auto_theme_idx += 1;
                if auto_theme_idx >= auto_themes.len() {
                    auto_theme_idx = 0;
                    auto_mode_idx += 1;
                }
                
                if auto_mode_idx >= auto_modes.len() {
                    println!("--- AUTOMATED BENCHMARK SEQUENCE COMPLETED ---");
                    std::process::exit(0);
                } else {
                    auto_timer = 0.0;
                    mode = auto_modes[auto_mode_idx];
                    theme = auto_themes[auto_theme_idx];
                    mode_changed = true;
                }
            }
        } else {
            // Cursor toggle
            if is_key_pressed(KeyCode::Escape) {
                cursor_grabbed = !cursor_grabbed;
                set_cursor_grab(cursor_grabbed);
                show_mouse(!cursor_grabbed);
            }

            // FPS Controls
            if cursor_grabbed {
                let delta = mouse_delta_position();
                yaw += delta.x * 0.5;
                pitch -= delta.y * 0.5;
                pitch = pitch.clamp(-1.5, 1.5); // restrict look up/down

                let front = vec3(yaw.cos() * pitch.cos(), pitch.sin(), yaw.sin() * pitch.cos()).normalize();
                let right = front.cross(vec3(0.0, 1.0, 0.0)).normalize();
                
                let speed = 200.0 * dt;
                if is_key_down(KeyCode::W) { player_pos += front * speed; }
                if is_key_down(KeyCode::S) { player_pos -= front * speed; }
                if is_key_down(KeyCode::A) { player_pos -= right * speed; }
                if is_key_down(KeyCode::D) { player_pos += right * speed; }

                // Shoot
                if is_mouse_button_pressed(MouseButton::Left) {
                    projectiles.push(Projectile {
                        pos: player_pos,
                        dir: front,
                        active: true,
                    });
                }
            }
        }

        // Update Projectiles & Collisions
        for p in &mut projectiles {
            if p.active {
                p.pos += p.dir * 1000.0 * dt;
                for (i, boid) in sim.boids.iter().enumerate() {
                    if boid_active[i] {
                        let hover_y = (get_time() as f32 * 2.0 + (i as f32)).sin() * 10.0;
                        let boid_pos3d = vec3(boid.position.x, hover_y, boid.position.y);
                        if p.pos.distance(boid_pos3d) < 15.0 {
                            boid_active[i] = false;
                            p.active = false;
                            score += 100;
                            break;
                        }
                    }
                }
                if p.pos.distance(player_pos) > 2000.0 { p.active = false; }
            }
        }

        let mut sorted: Vec<f32> = frame_times.iter().copied().collect();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let avg = if sorted.is_empty() { 0.0 } else { sorted.iter().sum::<f32>() / sorted.len() as f32 };
        let one_percent_count = ((sorted.len() as f32 * 0.01).ceil() as usize).max(1);
        let slowest_1 = &sorted[sorted.len().saturating_sub(one_percent_count)..];
        let low1 = if slowest_1.is_empty() { 0.0 } else { slowest_1.iter().sum::<f32>() / slowest_1.len() as f32 };

        if !auto_bench {
            if is_key_pressed(KeyCode::M) {
                theme = match theme {
                    VisualTheme::Cubes => VisualTheme::VoxelTerrain,
                    VisualTheme::VoxelTerrain => VisualTheme::WireframeSpace,
                    VisualTheme::WireframeSpace => VisualTheme::Cubes,
                };
            }
            if is_key_pressed(KeyCode::C) { TAMPER_FLAG.store(true, Ordering::Relaxed); }
            if is_key_pressed(KeyCode::T) { token_crack_attempted = true; }
        }

        tampered = TAMPER_FLAG.load(Ordering::Relaxed) || token_crack_attempted;

        let sim_start = get_time();
        if !tampered {
            sim.step(1.0 / 60.0);
            let _ = profile.on_checkpoint();
        }
        let sim_time = (get_time() - sim_start) as f32 * 1000.0;
        
        if frame_times.len() >= 300 { frame_times.pop_front(); }
        frame_times.push_back(sim_time);

        clear_background(Color::new(0.05, 0.05, 0.1, 1.0));

        let front = vec3(yaw.cos() * pitch.cos(), pitch.sin(), yaw.sin() * pitch.cos()).normalize();
        set_camera(&Camera3D {
            position: player_pos,
            up: vec3(0.0, 1.0, 0.0),
            target: player_pos + front,
            ..Default::default()
        });

        for (i, boid) in sim.boids.iter().enumerate() {
            if !boid_active[i] { continue; }
            let color = if i % 3 == 0 { RED } else if i % 3 == 1 { GREEN } else { BLUE };
            let size = 6.0;
            let hover_y = (get_time() as f32 * 2.0 + (i as f32)).sin() * 10.0;
            
            match theme {
                VisualTheme::Cubes => {
                    draw_cube(vec3(boid.position.x, hover_y, boid.position.y), vec3(size, size, size), None, color);
                }
                VisualTheme::VoxelTerrain => {
                    draw_cube(vec3(boid.position.x, -20.0 + hover_y, boid.position.y), vec3(size*2.0, size*4.0, size*2.0), None, DARKGREEN);
                }
                VisualTheme::WireframeSpace => {
                    let p = vec3(boid.position.x, hover_y * 3.0, boid.position.y);
                    draw_line_3d(p + vec3(-size, 0.0, -size), p + vec3(size, 0.0, -size), WHITE);
                    draw_line_3d(p + vec3(size, 0.0, -size), p + vec3(0.0, 0.0, size), WHITE);
                    draw_line_3d(p + vec3(0.0, 0.0, size), p + vec3(-size, 0.0, -size), WHITE);
                }
            }
        }
        
        for p in &projectiles {
            if p.active { draw_sphere(p.pos, 2.0, None, YELLOW); }
        }

        set_default_camera();

        if cursor_grabbed {
            draw_line(W/2.0 - 10.0, H/2.0, W/2.0 + 10.0, H/2.0, 2.0, WHITE);
            draw_line(W/2.0, H/2.0 - 10.0, W/2.0, H/2.0 + 10.0, 2.0, WHITE);
        } else if !auto_bench {
            draw_text("PRESS ESC TO ENTER FPS MODE", W/2.0 - 200.0, H/2.0 - 50.0, 30.0, YELLOW);
        }

        draw_rectangle(10.0, 10.0, 320.0, 340.0, Color::new(0.0, 0.0, 0.0, 0.8));
        draw_text("3D DRM FPS TESTBED", 20.0, 35.0, 24.0, WHITE);
        draw_text(&format!("Score: {}", score), 20.0, 60.0, 24.0, ORANGE);
        draw_text(&format!("FPS: {}", get_fps()), 20.0, 85.0, 20.0, WHITE);
        draw_text(&format!("Avg Sim: {:.2} ms", avg), 20.0, 105.0, 20.0, WHITE);
        draw_text(&format!("1% Low Sim: {:.2} ms", low1), 20.0, 125.0, 20.0, RED);

        draw_text("DRM Control Dashboard:", 20.0, 155.0, 16.0, GRAY);
        
        if !auto_bench {
            if draw_button(20.0, 165.0, 130.0, 30.0, "1. Baseline", matches!(mode, Mode::Baseline), cursor_grabbed) || is_key_pressed(KeyCode::Key1) {
                if !matches!(mode, Mode::Baseline) { log_session(mode.name(), avg, low1); mode = Mode::Baseline; mode_changed = true; }
            }
            if draw_button(160.0, 165.0, 130.0, 30.0, "2. InGen", matches!(mode, Mode::InGen), cursor_grabbed) || is_key_pressed(KeyCode::Key2) {
                if !matches!(mode, Mode::InGen) { log_session(mode.name(), avg, low1); mode = Mode::InGen; mode_changed = true; }
            }
            if draw_button(20.0, 200.0, 130.0, 30.0, "3. Heavy-R", matches!(mode, Mode::HeavyReasonable), cursor_grabbed) || is_key_pressed(KeyCode::Key3) {
                if !matches!(mode, Mode::HeavyReasonable) { log_session(mode.name(), avg, low1); mode = Mode::HeavyReasonable; mode_changed = true; }
            }
            if draw_button(160.0, 200.0, 130.0, 30.0, "4. Heavy-A", matches!(mode, Mode::HeavyAbusive), cursor_grabbed) || is_key_pressed(KeyCode::Key4) {
                if !matches!(mode, Mode::HeavyAbusive) { log_session(mode.name(), avg, low1); mode = Mode::HeavyAbusive; mode_changed = true; }
            }
            if draw_button(20.0, 235.0, 130.0, 30.0, "5. Online", matches!(mode, Mode::AlwaysOnline), cursor_grabbed) || is_key_pressed(KeyCode::Key5) {
                if !matches!(mode, Mode::AlwaysOnline) { log_session(mode.name(), avg, low1); mode = Mode::AlwaysOnline; mode_changed = true; }
            }
            if draw_button(160.0, 235.0, 130.0, 30.0, "6. VM-Hash", matches!(mode, Mode::VMAntitamper), cursor_grabbed) || is_key_pressed(KeyCode::Key6) {
                if !matches!(mode, Mode::VMAntitamper) { log_session(mode.name(), avg, low1); mode = Mode::VMAntitamper; mode_changed = true; }
            }

            if draw_button(20.0, 275.0, 270.0, 30.0, &format!("Theme (M): {}", theme.name()), false, cursor_grabbed) {
                theme = match theme {
                    VisualTheme::Cubes => VisualTheme::VoxelTerrain,
                    VisualTheme::VoxelTerrain => VisualTheme::WireframeSpace,
                    VisualTheme::WireframeSpace => VisualTheme::Cubes,
                };
            }
            draw_text("Crack Simulator (Keys): 'C' = RAM Tamper, 'T' = Token", 20.0, 325.0, 14.0, ORANGE);
        }

        draw_rectangle(W - 460.0, 10.0, 450.0, 180.0, Color::new(0.0, 0.1, 0.0, 0.8));
        draw_text("LIVE SECURITY TELEMETRY", W - 440.0, 35.0, 20.0, GREEN);
        
        if let Ok(log_content) = fs::read_to_string("security_audit.log") {
            let lines: Vec<&str> = log_content.lines().collect();
            let mut display_lines = Vec::new();
            for i in (0..lines.len()).rev().take(7) { display_lines.push(lines[i]); }
            display_lines.reverse();
            
            let mut y_offset = 60.0;
            for line in display_lines {
                let color = if line.contains("[CRITICAL]") { RED } else { LIGHTGRAY };
                draw_text(line, W - 440.0, y_offset, 14.0, color);
                y_offset += 16.0;
            }
        }

        if mode_changed {
            sim = Simulation::new(BOID_COUNT, W, H, 12345);
            TAMPER_FLAG.store(false, Ordering::Relaxed);
            token_crack_attempted = false;
            tampered = false;
            boid_active = vec![true; BOID_COUNT];
            
            profile = create_profile(&mode);
            let _ = profile.on_startup();
            let _ = profile.load_protected_asset();
            frame_times.clear();
        }

        if tampered && !auto_bench {
            draw_rectangle(0.0, 0.0, W, H, Color::new(1.0, 0.0, 0.0, 0.4));
            let msg = if token_crack_attempted { "DRM VIOLATION: INVALID LICENSE TOKEN SIGNATURE!" } 
                      else { "DRM VIOLATION: MEMORY TAMPERING DETECTED!" };
            let text_size = measure_text(msg, None, 40, 1.0);
            draw_text(msg, W / 2.0 - text_size.width / 2.0, H / 2.0, 40.0, WHITE);
            draw_text("Gameplay Halted. Press '1' to reset.", W / 2.0 - 200.0, H / 2.0 + 40.0, 30.0, WHITE);
        }

        if auto_bench {
            // Draw a big overlay saying Auto Benchmarking OVER everything
            draw_rectangle(0.0, 0.0, W, H, Color::new(0.0, 0.0, 0.0, 0.5));
            draw_text("AUTOMATED DATA COLLECTION IN PROGRESS...", W/2.0 - 400.0, H/2.0 - 50.0, 40.0, RED);
            draw_text(&format!("Testing: {} + {}", mode.name(), theme.name()), W/2.0 - 300.0, H/2.0 + 20.0, 30.0, WHITE);
        }

        next_frame().await;
    }
}
