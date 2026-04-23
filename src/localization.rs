use crate::Vector3;

/// A spherical constraint in 3-D space:
///
/// ```text
/// ‖x − centre‖ = radius
/// ```
///
/// This single type covers both problem domains from the equivalence:
///
/// ```text
/// ‖x − sᵢ‖ = c·Δtᵢ          (TDOA / TOA geolocation: centre = sensor sᵢ, radius = c·Δtᵢ)
/// ‖profile − aᵢ‖ = rᵢ(βᵢ)   (profile matching: centre = anchor aᵢ, radius = rᵢ(βᵢ))
/// ```
#[derive(Clone, Copy, Debug)]
pub struct SphericalConstraint {
    pub centre: Vector3,
    pub radius: f64,
}

impl SphericalConstraint {
    /// Construct a geolocation constraint from a sensor position and a
    /// measured travel time `delta_t` at signal propagation speed `c`.
    pub fn from_toa(sensor: Vector3, delta_t: f64, c: f64) -> Self {
        SphericalConstraint {
            centre: sensor,
            radius: c * delta_t,
        }
    }

    /// Signed residual of point `p` with respect to this constraint.
    /// Zero means the point lies exactly on the sphere.
    pub fn residual(&self, p: Vector3) -> f64 {
        distance(p, self.centre) - self.radius
    }
}

/// One of the four "Black Ferrari" sensors / anchors that together constrain
/// the position of the Angel.  Each Ferrari contributes one `SphericalConstraint`.
#[derive(Clone, Debug)]
pub struct BlackFerrari {
    pub id: u32,
    pub constraint: SphericalConstraint,
}

impl BlackFerrari {
    pub fn new(id: u32, constraint: SphericalConstraint) -> Self {
        BlackFerrari { id, constraint }
    }
}

/// The located entity — the Angel — found at (or near) the intersection of
/// all four Black Ferrari constraints.
#[derive(Clone, Copy, Debug)]
pub struct Angel {
    pub position: Vector3,
    /// RMS residual across all constraints at convergence.  Smaller is better;
    /// zero means the point lies exactly on every sphere.
    pub rms_error: f64,
}

/// Locates the [`Angel`] by minimising the sum of squared spherical residuals
/// via gradient descent.
///
/// # Objective
///
/// ```text
/// f(x) = Σᵢ ( ‖x − cᵢ‖ − rᵢ )²
/// ```
///
/// Both problem domains in the problem statement share this objective:
/// * TDOA geolocation:  cᵢ = sᵢ (sensor),   rᵢ = c·Δtᵢ
/// * Profile matching:  cᵢ = aᵢ (anchor),   rᵢ = rᵢ(βᵢ)
pub struct EncounterSolver {
    pub ferraris: Vec<BlackFerrari>,
    /// Gradient-descent step size.
    pub learning_rate: f64,
    /// Maximum number of iterations before returning best-so-far solution.
    pub max_iter: usize,
    /// Convergence threshold on the per-step displacement of `x`.
    pub tolerance: f64,
}

impl EncounterSolver {
    /// Create a solver for exactly four [`BlackFerrari`] detectors.
    pub fn new(ferraris: Vec<BlackFerrari>) -> Self {
        EncounterSolver {
            ferraris,
            learning_rate: 0.1,
            max_iter: 2000,
            tolerance: 1e-10,
        }
    }

    /// Run gradient descent from `initial_guess` and return the [`Angel`].
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

    /// Gradient of the objective and the current RMS residual.
    ///
    /// ∂f/∂x = 2 · Σᵢ residualᵢ · (x − cᵢ) / ‖x − cᵢ‖
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

fn distance(a: Vector3, b: Vector3) -> f64 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    let dz = a.z - b.z;
    (dx * dx + dy * dy + dz * dz).sqrt()
}
