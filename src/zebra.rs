/// Village role: the pasture keeper's coat-pattern classification mark —
/// denotes how a zebra's stripes run, used to identify family lines in the
/// herd registry.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum StripePattern {
    /// Stripes run parallel to the ground — a lowland lineage marker.
    Horizontal,
    /// Stripes run head-to-tail — the most common highland marking.
    Vertical,
    /// Stripes run at an angle — a rare cross-bred marking.
    Diagonal,
}

/// Village role: an individual zebra registered in the pasture roll — the
/// ledger records its sprint speed, bioelectric charge, coat pattern, and
/// remaining stamina so the herd keeper can track fitness and alarm state.
///
/// The ⚡ charge field models the low-level bioelectric potential
/// that zebras use for muscle signalling and inter-herd coordination.
#[derive(Clone, Copy, Debug)]
pub struct Zebra {
    /// Unique identifier assigned by the pasture registry.
    pub id: u32,
    /// Current sprint speed in m/s (top field speed ~16 m/s).
    pub speed: f32,
    /// Bioelectric membrane potential in millivolts; resting value ≈ 70 mV.
    pub charge_mv: f32,
    /// Coat-pattern classification from the herd registry.
    pub stripe_pattern: StripePattern,
    /// Remaining endurance fraction [0.0 = exhausted, 1.0 = fully rested].
    pub stamina: f32,
}

impl Zebra {
    /// Village role: register a new zebra in the pasture roll with default
    /// resting values (speed = 0, charge = 70 mV, stamina = 1.0).
    pub fn new(id: u32, stripe_pattern: StripePattern) -> Self {
        Zebra {
            id,
            speed: 0.0,
            charge_mv: 70.0,
            stripe_pattern,
            stamina: 1.0,
        }
    }

    /// Village role: the sprint command — accelerate toward `target_speed`
    /// over `dt` seconds, draining stamina in proportion to the effort applied.
    pub fn accelerate(&mut self, target_speed: f32, dt: f32) {
        let delta = (target_speed - self.speed).clamp(-20.0, 20.0);
        self.speed += delta * dt;
        let exertion = delta.abs() * dt * 0.05;
        self.stamina = (self.stamina - exertion).max(0.0);
    }

    /// Village role: the recovery order — slow the zebra to a halt and
    /// replenish stamina at the pasture rest rate over `dt` seconds.
    pub fn rest(&mut self, dt: f32) {
        self.stamina = (self.stamina + dt * 0.1).min(1.0);
        self.speed = (self.speed - dt * 5.0).max(0.0);
    }

    /// Village role: the alarm spike — the zebra fires its bioelectric pulse
    /// (returning the peak voltage to the caller) and resets to resting
    /// potential, signalling danger to the rest of the herd.
    pub fn discharge(&mut self) -> f32 {
        let spike = self.charge_mv;
        self.charge_mv = 70.0;
        spike
    }

    /// Village role: the sprint-status flag — reports whether this zebra is
    /// currently galloping above the alarm-sprint threshold (12 m/s).
    pub fn is_sprinting(&self) -> bool {
        self.speed > 12.0
    }
}

/// Village role: the pasture collective — the full zebra herd managed as a
/// single unit.  The herd keeper ticks all members together; if any one
/// member fires an alarm, the entire herd bolts.
pub struct ZebraHerd {
    /// All zebras currently registered in this herd.
    pub members: Vec<Zebra>,
}

impl ZebraHerd {
    /// Village role: open an empty pasture — no zebras enrolled yet.
    pub fn new() -> Self {
        ZebraHerd {
            members: Vec::new(),
        }
    }

    /// Village role: enrol a new zebra in the herd registry.
    pub fn add(&mut self, zebra: Zebra) {
        self.members.push(zebra);
    }

    /// Village role: the pasture clock tick — advances all herd members by
    /// `dt` seconds.  If any member's charge meets or exceeds
    /// `alarm_threshold_mv`, the whole herd receives a sprint command;
    /// otherwise fatigued members are sent to rest.
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

    /// Village role: the herd speed gauge — returns the mean sprint speed
    /// (m/s) across all enrolled members, or 0.0 for an empty herd.
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
