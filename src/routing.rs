use crate::{Trajectory, Vector3};

/// A single stop along a planned route.
#[derive(Clone, Copy)]
pub struct Waypoint {
    pub position: Vector3,
    pub arrival_time: f64,
}

/// An ordered sequence of waypoints describing a planned path.
pub struct Route {
    pub waypoints: Vec<Waypoint>,
    pub total_delta_v: f64,
}

impl Route {
    pub fn new() -> Self {
        Route {
            waypoints: Vec::new(),
            total_delta_v: 0.0,
        }
    }

    pub fn push(&mut self, waypoint: Waypoint, delta_v: f64) {
        self.waypoints.push(waypoint);
        self.total_delta_v += delta_v;
    }

    pub fn len(&self) -> usize {
        self.waypoints.len()
    }

    pub fn is_empty(&self) -> bool {
        self.waypoints.is_empty()
    }
}

impl Default for Route {
    fn default() -> Self {
        Route::new()
    }
}

/// Selects and sequences trajectories to form an efficient route.
pub struct Router {
    pub trajectories: Vec<Trajectory>,
}

impl Router {
    pub fn new(trajectories: Vec<Trajectory>) -> Self {
        Router { trajectories }
    }

    /// Returns the trajectory with the lowest delta-v that departs from
    /// a position within `tolerance` of `origin`.
    pub fn cheapest_from(&self, origin: Vector3, tolerance: f64) -> Option<&Trajectory> {
        self.trajectories
            .iter()
            .filter(|t| distance(t.origin, origin) <= tolerance)
            .min_by(|a, b| a.delta_v.partial_cmp(&b.delta_v).unwrap())
    }

    /// Greedily builds a route from `start` to `goal`, chaining trajectories
    /// whose origins lie within `tolerance` of the previous destination.
    /// Returns `None` if no path reaches the goal within `max_hops` steps.
    pub fn plan(
        &self,
        start: Vector3,
        goal: Vector3,
        tolerance: f64,
        max_hops: usize,
    ) -> Option<Route> {
        let mut route = Route::new();
        let mut current = start;
        let mut arrival = 0.0_f64;

        for _ in 0..max_hops {
            if distance(current, goal) <= tolerance {
                route.push(
                    Waypoint {
                        position: current,
                        arrival_time: arrival,
                    },
                    0.0,
                );
                return Some(route);
            }

            let traj = self
                .trajectories
                .iter()
                .filter(|t| distance(t.origin, current) <= tolerance)
                .min_by(|a, b| {
                    let da = distance(a.destination, goal);
                    let db = distance(b.destination, goal);
                    da.partial_cmp(&db).unwrap()
                })?;

            route.push(
                Waypoint {
                    position: current,
                    arrival_time: arrival,
                },
                traj.delta_v,
            );

            arrival += traj.delta_v;
            current = traj.destination;
        }

        None
    }
}

fn distance(a: Vector3, b: Vector3) -> f64 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    let dz = a.z - b.z;
    (dx * dx + dy * dy + dz * dz).sqrt()
}
