use crate::Vector3;

/// Village role: a range ring drawn on the village map from one watchtower —
/// represents the single constraint
///
/// ```text
/// ‖x − centre‖ = radius
/// ```
///
/// This one type covers both domains of the encounter equation:
///
/// ```text
/// ‖x − sᵢ‖ = c·Δtᵢ          (TDOA / TOA geolocation: centre = sensor sᵢ, radius = c·Δtᵢ)
/// ‖profile − aᵢ‖ = rᵢ(βᵢ)   (profile matching: centre = anchor aᵢ, radius = rᵢ(βᵢ))
/// ```
#[derive(Clone, Copy, Debug)]
pub struct SphericalConstraint {
    /// Map position of the watchtower (sensor or anchor).
    pub centre: Vector3,
    /// Radius of the range ring on the village map (metres).
    pub radius: f64,
}

impl SphericalConstraint {
    /// Village role: the time-of-arrival scribe — constructs a range ring
    /// from a sensor position and a measured signal travel time `delta_t`
    /// at propagation speed `c` (radius = c · Δt).
    pub fn from_toa(sensor: Vector3, delta_t: f64, c: f64) -> Self {
        SphericalConstraint {
            centre: sensor,
            radius: c * delta_t,
        }
    }

    /// Village role: the ring-distance inspector — returns how far point `p`
    /// is from lying exactly on this range ring.  Zero means perfect
    /// intersection; positive means outside the ring; negative means inside.
    pub fn residual(&self, p: Vector3) -> f64 {
        distance(p, self.centre) - self.radius
    }
}

/// Village role: one of the four Black Ferrari sentinels posted around the
/// village perimeter — each sentinel draws its own range ring on the map,
/// and together the four rings uniquely fix the Angel's position.
#[derive(Clone, Debug)]
pub struct BlackFerrari {
    /// Sentinel identifier in the watch roster (0–3 for the canonical four).
    pub id: u32,
    /// The range ring this sentinel contributes to the encounter solution.
    pub constraint: SphericalConstraint,
}

impl BlackFerrari {
    /// Village role: post a new sentinel at its assigned watch position with
    /// its pre-computed range ring.
    pub fn new(id: u32, constraint: SphericalConstraint) -> Self {
        BlackFerrari { id, constraint }
    }
}

/// Village role: the Angel — the visitor whose position is unknown and must
/// be resolved by the encounter solver from the four sentinels' range rings.
/// Once located, the Angel's entry is stamped into the ledger with a
/// precision score (`rms_error`).
#[derive(Clone, Copy, Debug)]
pub struct Angel {
    /// Resolved position on the village map.
    pub position: Vector3,
    /// RMS residual across all four range rings at convergence — measures
    /// how precisely the Angel was pinned.  Zero = exact intersection.
    pub rms_error: f64,
}

/// Village role: the village triangulation scribe — receives the four
/// [`BlackFerrari`] sentinels' range rings and runs a gradient-descent
/// ritual to find the point on the map that lies (as close as possible)
/// on all four rings simultaneously, yielding the Angel's ledger entry.
///
/// # Objective minimised
///
/// ```text
/// f(x) = Σᵢ ( ‖x − cᵢ‖ − rᵢ )²
/// ```
///
/// Both problem domains share this objective:
/// * TDOA geolocation:  cᵢ = sᵢ (sensor),   rᵢ = c·Δtᵢ
/// * Profile matching:  cᵢ = aᵢ (anchor),   rᵢ = rᵢ(βᵢ)
pub struct EncounterSolver {
    /// The four sentinels whose range rings constrain the Angel's position.
    pub ferraris: Vec<BlackFerrari>,
    /// Gradient-descent step size — the scribe's stride at each iteration.
    pub learning_rate: f64,
    /// Maximum iterations before the scribe records the best-so-far answer.
    pub max_iter: usize,
    /// Convergence threshold on the per-step displacement of `x` (metres).
    pub tolerance: f64,
}

impl EncounterSolver {
    /// Village role: open a new triangulation ritual with a given set of
    /// [`BlackFerrari`] sentinels (canonically four).
    pub fn new(ferraris: Vec<BlackFerrari>) -> Self {
        EncounterSolver {
            ferraris,
            learning_rate: 0.1,
            max_iter: 2000,
            tolerance: 1e-10,
        }
    }

    /// Village role: run the triangulation ritual — iterates gradient descent
    /// from `initial_guess` until converged or the iteration budget is
    /// exhausted, then stamps the resolved [`Angel`] into the ledger.
    pub fn solve(&self, initial_guess: Vector3) -> Angel {
        let mut x = initial_guess;

        for _ in 0..self.max_iter {
            let (grad, _) = self.gradient_and_rms(x);
            let new_x = Vector3 {
                x: x.x - self.learning_rate * grad.x,
                y: x.y - self.learning_rate * grad.y,
                z: x.z - self.learning_rate * grad.z,
            };

            let moved = distance(new_x, x);
            x = new_x;

            if moved < self.tolerance {
                break;
            }
        }

        let (_, rms) = self.gradient_and_rms(x);
        Angel {
            position: x,
            rms_error: rms,
        }
    }

    /// Village role: the scribe's inner calculus — computes the gradient of
    /// the objective at `x` and the current RMS residual across all sentinels.
    ///
    /// `∂f/∂x = 2 · Σᵢ residualᵢ · (x − cᵢ) / ‖x − cᵢ‖`
    fn gradient_and_rms(&self, x: Vector3) -> (Vector3, f64) {
        let n = self.ferraris.len() as f64;
        let mut gx = 0.0_f64;
        let mut gy = 0.0_f64;
        let mut gz = 0.0_f64;
        let mut sum_sq = 0.0_f64;

        for ferrari in &self.ferraris {
            let c = ferrari.constraint;
            let dist = distance(x, c.centre);
            if dist < 1e-12 {
                continue;
            }
            let residual = dist - c.radius;
            sum_sq += residual * residual;
            let scale = 2.0 * residual / dist;
            gx += scale * (x.x - c.centre.x);
            gy += scale * (x.y - c.centre.y);
            gz += scale * (x.z - c.centre.z);
        }

        let rms = if n > 0.0 { (sum_sq / n).sqrt() } else { 0.0 };
        (Vector3 { x: gx, y: gy, z: gz }, rms)
    }
}

/// Village role: the cartographer's ruler — computes the straight-line distance
/// between two points on the village map (Euclidean norm in 3-D).
fn distance(a: Vector3, b: Vector3) -> f64 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    let dz = a.z - b.z;
    (dx * dx + dy * dy + dz * dz).sqrt()
}
