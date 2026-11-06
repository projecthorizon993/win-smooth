//! Physics-based spring animation (damped spring, Hooke's law).
//! No easing functions — spring stiffness + damping only. 60fps tick.

/// Spring animation state for a single animated property.
#[derive(Debug, Clone, Copy)]
pub struct Spring {
    pub value: f32,
    pub target: f32,
    pub velocity: f32,
    pub stiffness: f32,
    pub damping: f32,
}

impl Spring {
    pub fn new(start: f32) -> Self {
        Self {
            value: start,
            target: start,
            velocity: 0.0,
            stiffness: 120.0,
            damping: 28.0,
        }
    }

    pub fn with_tuning(mut self, stiffness: f32, damping: f32) -> Self {
        self.stiffness = stiffness;
        self.damping = damping;
        self
    }

    pub fn set_target(&mut self, target: f32) {
        self.target = target;
    }

    /// Advance one frame (dt in seconds). Returns true if still animating.
    pub fn update(&mut self, dt: f32) -> bool {
        let displacement = self.target - self.value;
        let acceleration = self.stiffness * displacement - self.damping * self.velocity;
        self.velocity += acceleration * dt;
        self.value += self.velocity * dt;

        let still_moving = displacement.abs() > 0.5 || self.velocity.abs() > 2.0;
        still_moving
    }

    pub fn is_settled(&self) -> bool {
        (self.target - self.value).abs() < 0.5 && self.velocity.abs() < 2.0
    }
}

/// Animation manager: tracks multiple springs, ticks at 60fps.
pub struct AnimationManager {
    springs: Vec<(String, Spring)>,
    last_time: Option<std::time::Instant>,
}

impl AnimationManager {
    pub fn new() -> Self {
        Self {
            springs: Vec::new(),
            last_time: None,
        }
    }

    pub fn add(&mut self, name: &str, start: f32) {
        self.springs.push((name.to_string(), Spring::new(start)));
    }

    pub fn set_target(&mut self, name: &str, target: f32) {
        if let Some((_, spring)) = self.springs.iter_mut().find(|(n, _)| n == name) {
            spring.set_target(target);
        }
    }

    pub fn value(&self, name: &str) -> Option<f32> {
        self.springs.iter().find(|(n, _)| n == name).map(|(_, s)| s.value)
    }

    /// Tick all springs. Returns true if any spring is still animating.
    pub fn tick(&mut self) -> bool {
        let now = std::time::Instant::now();
        let dt = self
            .last_time
            .map(|t| now.duration_since(t).as_secs_f32())
            .unwrap_or(1.0 / 60.0);
        self.last_time = Some(now);

        let mut any_moving = false;
        for (_, spring) in self.springs.iter_mut() {
            if spring.update(dt) {
                any_moving = true;
            }
        }
        any_moving
    }

    pub fn is_settled(&self) -> bool {
        self.springs.iter().all(|(_, s)| s.is_settled())
    }
}

impl Default for AnimationManager {
    fn default() -> Self {
        Self::new()
    }
}
