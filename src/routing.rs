use crate::{Trajectory, Vector3};

/// Village role: a waystation on the village road network — records the
/// precise map position of a stop and the time at which a traveller is
/// expected to arrive there.
#[derive(Clone, Copy)]
pub struct Waypoint {
    /// Map position of this waystation.
    pub position: Vector3,
    /// Scheduled arrival time at this waystation (seconds since journey start).
    pub arrival_time: f64,
}

/// Village role: a traveller's itinerary filed with the village roads guild —
/// an ordered sequence of waystations together with the total propulsive cost
/// (Δv) accumulated along the entire journey.
pub struct Route {
    /// Ordered list of waystations from departure to destination.
    pub waypoints: Vec<Waypoint>,
    /// Cumulative Δv consumed by the journey so far (m/s).
    pub total_delta_v: f64,
}

impl Route {
    /// Village role: open a new blank itinerary at the roads guild counter.
    pub fn new() -> Self {
        Route {
            waypoints: Vec::new(),
            total_delta_v: 0.0,
        }
    }

    /// Village role: stamp a new waystation into the itinerary and add the
    /// leg's Δv cost to the running journey total.
    pub fn push(&mut self, waypoint: Waypoint, delta_v: f64) {
        self.waypoints.push(waypoint);
        self.total_delta_v += delta_v;
    }

    /// Village role: report how many waystations are currently logged in
    /// this itinerary.
    pub fn len(&self) -> usize {
        self.waypoints.len()
    }

    /// Village role: confirm whether the itinerary is still blank (no
    /// waystations logged yet).
    pub fn is_empty(&self) -> bool {
        self.waypoints.is_empty()
    }
}

impl Default for Route {
    fn default() -> Self {
        Route::new()
    }
}

/// Village role: the roads guild's master route-planner — holds every
/// approved trajectory leg and uses them to find efficient paths between
/// any two points on the village map.
pub struct Router {
    /// All trajectory legs available for route construction.
    pub trajectories: Vec<Trajectory>,
}

impl Router {
    /// Village role: charter a new route-planner with a given set of
    /// approved trajectory legs.
    pub fn new(trajectories: Vec<Trajectory>) -> Self {
        Router { trajectories }
    }

    /// Village role: the thrifty traveller's query — returns the single
    /// cheapest (lowest Δv) trajectory leg that departs from a position
    /// within `tolerance` of `origin`, or `None` if no such leg exists.
    pub fn cheapest_from(&self, origin: Vector3, tolerance: f64) -> Option<&Trajectory> {
        self.trajectories
            .iter()
            .filter(|t| distance(t.origin, origin) <= tolerance)
            .min_by(|a, b| a.delta_v.partial_cmp(&b.delta_v).unwrap())
    }

    /// Village role: the guild scribe's greedy path-finding ritual — chains
    /// trajectory legs from `start` toward `goal`, always picking the leg
    /// whose destination is closest to the goal.  Returns the completed
    /// [`Route`] itinerary, or `None` if no path is found within `max_hops`.
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

/// Village role: the cartographer's distance formula — computes the straight-line
/// distance between two points on the village map (Euclidean norm in 3-D).
fn distance(a: Vector3, b: Vector3) -> f64 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    let dz = a.z - b.z;
    (dx * dx + dy * dy + dz * dz).sqrt()
}
