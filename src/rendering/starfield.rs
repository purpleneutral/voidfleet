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

/// Parallax scrolling star field.
#[derive(Debug)]
pub struct Starfield {
    pub stars: Vec<Star>,
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
    pub fn tick(&mut self) {
        let mut rng = rand::thread_rng();
        for star in &mut self.stars {
            star.x -= star.speed;
            if star.x < 0.0 {
                *star = Self::random_star(&mut rng, self.width, self.height, true);
            }
        }
    }
}
