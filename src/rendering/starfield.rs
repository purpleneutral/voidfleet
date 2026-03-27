use rand::Rng;
use ratatui::style::Color;

/// A star in the parallax background.
#[derive(Debug, Clone)]
pub struct Star {
    pub x: f32,
    pub y: f32,
    pub speed: f32, // pixels per tick (higher = closer/faster)
    pub ch: char,
    pub color: Color,
}

/// A fast-moving diagonal streak across the sky.
#[derive(Debug, Clone)]
pub struct ShootingStar {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub life: u8,
    pub trail: Vec<(f32, f32)>, // previous positions for trail rendering
}

impl ShootingStar {
    fn new(rng: &mut impl Rng, width: u16, height: u16) -> Self {
        // Spawn in the upper-right quadrant, travel to bottom-left
        let x = rng.gen_range(width as f32 * 0.3..width as f32 + 5.0);
        let y = rng.gen_range(0.0..height as f32 * 0.4);
        let speed = rng.gen_range(1.5..3.0);
        Self {
            x,
            y,
            vx: -speed,
            vy: speed * 0.4, // shallower angle for terminal aspect ratio
            life: rng.gen_range(10..16),
            trail: Vec::with_capacity(5),
        }
    }

    fn tick(&mut self) -> bool {
        if self.life == 0 {
            return false;
        }
        // Record current position in trail before moving
        self.trail.push((self.x, self.y));
        if self.trail.len() > 5 {
            self.trail.remove(0);
        }
        self.x += self.vx;
        self.y += self.vy;
        self.life -= 1;
        true
    }
}

/// Parallax scrolling star field with shooting stars.
#[derive(Debug)]
pub struct Starfield {
    pub stars: Vec<Star>,
    pub shooting_stars: Vec<ShootingStar>,
    width: u16,
    height: u16,
}

impl Starfield {
    pub fn new(width: u16, height: u16, density: usize) -> Self {
        let mut rng = rand::thread_rng();
        let stars = (0..density)
            .map(|_| Self::random_star(&mut rng, width, height, false))
            .collect();
        Self {
            stars,
            shooting_stars: Vec::new(),
            width,
            height,
        }
    }

    fn random_star(rng: &mut impl Rng, width: u16, height: u16, from_right: bool) -> Star {
        let layer: u8 = rng.gen_range(0..3);
        let (speed, ch, color) = match layer {
            0 => (0.1, '.', Color::DarkGray),  // far
            1 => (0.3, '·', Color::Gray),      // mid
            _ => (0.6, '✦', Color::White),     // near
        };
        Star {
            x: if from_right {
                width as f32 + rng.gen_range(0.0..10.0)
            } else {
                rng.gen_range(0.0..width as f32)
            },
            y: rng.gen_range(0.0..height as f32),
            speed,
            ch,
            color,
        }
    }

    /// Resize the star field (terminal resize).
    pub fn resize(&mut self, width: u16, height: u16) {
        self.width = width;
        self.height = height;
    }

    /// Scroll all stars left, respawn those that go off-screen.
    /// Also ticks shooting stars and occasionally spawns new ones.
    pub fn tick(&mut self) {
        let mut rng = rand::thread_rng();

        // Regular stars
        for star in &mut self.stars {
            star.x -= star.speed;
            if star.x < 0.0 {
                *star = Self::random_star(&mut rng, self.width, self.height, true);
            }
        }

        // Tick existing shooting stars, remove dead ones
        self.shooting_stars.retain_mut(|s| s.tick());

        // 1% chance per tick to spawn a new shooting star (cap at 2 simultaneous)
        if self.shooting_stars.len() < 2
            && self.width > 10
            && self.height > 5
            && rng.gen_range(0..100) == 0
        {
            self.shooting_stars
                .push(ShootingStar::new(&mut rng, self.width, self.height));
        }
    }

    /// Get shooting star render data: returns (x, y, char, color) tuples
    /// for each visible pixel of each shooting star (head + trail).
    pub fn shooting_star_cells(&self) -> Vec<(u16, u16, char, Color)> {
        let mut cells = Vec::new();
        for star in &self.shooting_stars {
            // Head — bright white
            let hx = star.x.round() as i32;
            let hy = star.y.round() as i32;
            if hx >= 0 && hx < self.width as i32 && hy >= 0 && hy < self.height as i32 {
                cells.push((hx as u16, hy as u16, '★', Color::White));
            }

            // Trail — fading from bright to dim
            let trail_chars = ['─', '╌', '·', '.'];
            let trail_colors = [
                Color::Rgb(200, 200, 255),
                Color::Rgb(140, 140, 200),
                Color::Rgb(80, 80, 140),
                Color::DarkGray,
            ];
            for (i, &(tx, ty)) in star.trail.iter().rev().enumerate() {
                if i >= trail_chars.len() {
                    break;
                }
                let px = tx.round() as i32;
                let py = ty.round() as i32;
                if px >= 0 && px < self.width as i32 && py >= 0 && py < self.height as i32 {
                    cells.push((px as u16, py as u16, trail_chars[i], trail_colors[i]));
                }
            }
        }
        cells
    }
}
