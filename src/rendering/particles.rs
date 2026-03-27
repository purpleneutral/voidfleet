use ratatui::style::Color;

/// A single particle in the particle system.
#[derive(Debug, Clone)]
pub struct Particle {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub life: u8,
    pub max_life: u8,
    pub ch: char,
    pub color: Color,
}

impl Particle {
    pub fn new(x: f32, y: f32, vx: f32, vy: f32, life: u8, ch: char, color: Color) -> Self {
        Self {
            x,
            y,
            vx,
            vy,
            life,
            max_life: life,
            ch,
            color,
        }
    }

    /// Update position, decrement life. Returns false when dead.
    pub fn tick(&mut self) -> bool {
        if self.life == 0 {
            return false;
        }
        self.x += self.vx;
        self.y += self.vy;
        self.life -= 1;
        true
    }

    /// Alpha-like fade based on remaining life (1.0 = full, 0.0 = dead).
    pub fn fade(&self) -> f32 {
        self.life as f32 / self.max_life as f32
    }

    /// Get the character to render based on fade level.
    pub fn render_char(&self) -> char {
        let f = self.fade();
        if f > 0.7 {
            self.ch
        } else if f > 0.4 {
            '·'
        } else {
            '.'
        }
    }
}

/// Manages a collection of particles.
#[derive(Debug, Default)]
pub struct ParticleSystem {
    pub particles: Vec<Particle>,
}

impl ParticleSystem {
    pub fn new() -> Self {
        Self {
            particles: Vec::with_capacity(256),
        }
    }

    /// Update all particles, remove dead ones.
    pub fn tick(&mut self) {
        self.particles.retain_mut(|p| p.tick());
    }

    /// Add a single particle.
    pub fn emit(&mut self, p: Particle) {
        self.particles.push(p);
    }

    /// Emit an explosion burst at (x, y).
    pub fn explode(&mut self, x: f32, y: f32, count: usize, color: Color) {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let chars = ['*', '✦', '◆', '●', '▪', '∙'];
        for _ in 0..count {
            let angle: f32 = rng.gen_range(0.0..std::f32::consts::TAU);
            let speed: f32 = rng.gen_range(0.3..1.5);
            let life: u8 = rng.gen_range(5..15);
            let ch = chars[rng.gen_range(0..chars.len())];
            self.emit(Particle::new(
                x,
                y,
                angle.cos() * speed,
                angle.sin() * speed * 0.5, // terminal chars are taller than wide
                life,
                ch,
                color,
            ));
        }
    }

    /// Emit collection sparkle (resource picked up).
    pub fn sparkle(&mut self, x: f32, y: f32, color: Color) {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        for _ in 0..4 {
            self.emit(Particle::new(
                x,
                y,
                rng.gen_range(-0.3..0.3),
                rng.gen_range(-0.5..-0.1),
                rng.gen_range(4..8),
                '✦',
                color,
            ));
        }
    }

    /// Emit engine exhaust trail.
    pub fn exhaust(&mut self, x: f32, y: f32) {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        self.emit(Particle::new(
            x,
            y,
            rng.gen_range(-0.8..-0.3),
            rng.gen_range(-0.15..0.15),
            rng.gen_range(3..6),
            '░',
            Color::DarkGray,
        ));
    }
}
