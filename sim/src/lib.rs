use glam::Vec2;
use rand::{Rng, SeedableRng, RngExt};
use rand_chacha::ChaCha8Rng;
use std::f32::consts::TAU;

#[derive(Clone, Copy, Debug)]
pub struct Boid {
    pub position: Vec2,
    pub velocity: Vec2,
}

pub struct Simulation {
    pub boids: Vec<Boid>,
    pub width: f32,
    pub height: f32,
    pub perception_radius: f32,
    pub max_speed: f32,
    pub max_force: f32,
}

impl Simulation {
    pub fn new(num_boids: usize, width: f32, height: f32, seed: u64) -> Self {
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        let mut boids = Vec::with_capacity(num_boids);
        
        for _ in 0..num_boids {
            let position = Vec2::new(
                rng.random_range(0.0..width),
                rng.random_range(0.0..height),
            );
            let angle = rng.random_range(0.0..TAU);
            let speed = rng.random_range(10.0..50.0);
            let velocity = Vec2::new(angle.cos(), angle.sin()) * speed;
            
            boids.push(Boid { position, velocity });
        }
        
        Self {
            boids,
            width,
            height,
            perception_radius: 50.0,
            max_speed: 150.0,
            max_force: 5.0,
        }
    }

    #[inline(never)] // Ensure it's not totally inlined so we can potentially instrument it in heavy mode if we wanted, though DRM trait is separate.
    pub fn step(&mut self, dt: f32) {
        let boids_clone = self.boids.clone();
        
        for (i, boid) in self.boids.iter_mut().enumerate() {
            let mut alignment = Vec2::ZERO;
            let mut cohesion = Vec2::ZERO;
            let mut separation = Vec2::ZERO;
            let mut total = 0;
            
            for (j, other) in boids_clone.iter().enumerate() {
                if i == j { continue; }
                let d = boid.position.distance(other.position);
                if d < self.perception_radius && d > 0.0 {
                    alignment += other.velocity;
                    cohesion += other.position;
                    separation += (boid.position - other.position) / d;
                    total += 1;
                }
            }
            
            if total > 0 {
                let total_f = total as f32;
                alignment = (alignment / total_f).normalize_or_zero() * self.max_speed;
                let steer_align = (alignment - boid.velocity).clamp_length_max(self.max_force);
                
                cohesion = (cohesion / total_f - boid.position).normalize_or_zero() * self.max_speed;
                let steer_cohesion = (cohesion - boid.velocity).clamp_length_max(self.max_force);
                
                separation = separation.normalize_or_zero() * self.max_speed;
                let steer_separation = (separation - boid.velocity).clamp_length_max(self.max_force);
                
                boid.velocity += steer_align * 1.0 + steer_cohesion * 1.0 + steer_separation * 1.5;
            }
            
            boid.velocity = boid.velocity.clamp_length_max(self.max_speed);
            boid.position += boid.velocity * dt;
            
            if boid.position.x > self.width { boid.position.x -= self.width; }
            if boid.position.x < 0.0 { boid.position.x += self.width; }
            if boid.position.y > self.height { boid.position.y -= self.height; }
            if boid.position.y < 0.0 { boid.position.y += self.height; }
        }
    }
}
