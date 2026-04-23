/// Stripe orientation of a zebra's coat pattern.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum StripePattern {
    Horizontal,
    Vertical,
    Diagonal,
}

/// A single zebra with bioelectric and kinetic properties.
///
/// The ⚡ charge field models the low-level bioelectric potential
/// that zebras use for muscle signalling and inter-herd coordination.
#[derive(Clone, Copy, Debug)]
pub struct Zebra {
    pub id: u32,
    /// Current sprint speed in m/s (top field speed ~16 m/s).
    pub speed: f32,
    /// Bioelectric charge in millivolts.
    pub charge_mv: f32,
    pub stripe_pattern: StripePattern,
    pub stamina: f32,
}

impl Zebra {
    pub fn new(id: u32, stripe_pattern: StripePattern) -> Self {
        Zebra {
            id,
            speed: 0.0,
            charge_mv: 70.0,
            stripe_pattern,
            stamina: 1.0,
        }
    }

    /// Accelerate toward `target_speed`, draining stamina proportionally.
    pub fn accelerate(&mut self, target_speed: f32, dt: f32) {
        let delta = (target_speed - self.speed).clamp(-20.0, 20.0);
        self.speed += delta * dt;
        let exertion = delta.abs() * dt * 0.05;
        self.stamina = (self.stamina - exertion).max(0.0);
    }

    /// Replenish stamina when the zebra is at rest.
    pub fn rest(&mut self, dt: f32) {
        self.stamina = (self.stamina + dt * 0.1).min(1.0);
        self.speed = (self.speed - dt * 5.0).max(0.0);
    }

    /// Discharge a bioelectric pulse (e.g., alarm signal), resetting charge
    /// to resting potential after spiking.
    pub fn discharge(&mut self) -> f32 {
        let spike = self.charge_mv;
        self.charge_mv = 70.0;
        spike
    }

    pub fn is_sprinting(&self) -> bool {
        self.speed > 12.0
    }
}

/// A group of zebras that can be ticked as a unit.
pub struct ZebraHerd {
    pub members: Vec<Zebra>,
}

impl ZebraHerd {
    pub fn new() -> Self {
        ZebraHerd {
            members: Vec::new(),
        }
    }

    pub fn add(&mut self, zebra: Zebra) {
        self.members.push(zebra);
    }

    /// Advance all zebra states by `dt` seconds.
    /// If any member discharges an alarm (charge > threshold), the whole herd bolts.
    pub fn tick(&mut self, dt: f32, alarm_threshold_mv: f32) {
        let alarm = self
            .members
            .iter()
            .any(|z| z.charge_mv >= alarm_threshold_mv);

        for zebra in &mut self.members {
            if alarm {
                zebra.accelerate(16.0, dt);
            } else if zebra.stamina < 0.3 {
                zebra.rest(dt);
            }
        }
    }

    pub fn average_speed(&self) -> f32 {
        if self.members.is_empty() {
            return 0.0;
        }
        self.members.iter().map(|z| z.speed).sum::<f32>() / self.members.len() as f32
    }
}

impl Default for ZebraHerd {
    fn default() -> Self {
        ZebraHerd::new()
    }
}
