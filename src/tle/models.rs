use std::ops::Add;

use chrono::{DateTime, Utc};
use serde::Deserialize;

// Earth parameters (WGS-84)
pub const MU_EARTH_KM3_S2: f64 = 398600.4418; // km^3/s^2
pub const EARTH_RADIUS_KM: f64 = 6378.137; // km

#[derive(Debug, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct SatTle {
    pub object_name: String,
    pub object_id: String,
    pub epoch: String,
    pub mean_motion: f64,
    pub eccentricity: f64,
    pub inclination: f64,
    pub ra_of_asc_node: f64,
    pub arg_of_pericenter: f64,
    pub mean_anomaly: f64,
    pub ephemeris_type: usize,
    pub classification_type: String,
    pub norad_cat_id: usize,
    pub element_set_no: usize,
    pub rev_at_epoch: usize,
    pub bstar: usize,
    pub mean_motion_dot: f64,
    pub mean_motion_ddot: usize
}

#[derive(Debug)]
pub struct TleElements {
    pub name: String,
    pub epoch : DateTime<Utc>,
    pub a_km: f64,
    pub e: f64,
    pub i_rad: f64,
    pub raan_rad: f64,
    pub argp_rad: f64,
    pub m0_rad: f64,
    pub n_rad_s: f64
}

impl TleElements {
    pub fn new(tle: &SatTle) -> Self {
        let epoch_iso = &tle.epoch.clone().add("Z");
        let epoch = DateTime::parse_from_rfc3339(epoch_iso).unwrap();

        let i_deg = &tle.inclination;
        let e_deg = &tle.eccentricity;
        let raan_deg = &tle.ra_of_asc_node;
        let argp_deg = &tle.arg_of_pericenter;
        let mean_anom_deg = &tle.mean_anomaly;
        let n_rev_day = &tle.mean_motion;

        let i_rad = i_deg.to_radians();
        let raan_rad = raan_deg.to_radians();
        let argp_rad = argp_deg.to_radians();
        let m0_rad = mean_anom_deg.to_radians();

        // Mean motion [rev/day] → [rad/s]
        let n_rad_s = 2.0 * std::f64::consts::PI * n_rev_day / 86400.0;

        // Kepler's 3rd law: n^2 a^3 = mu
        let a = (MU_EARTH_KM3_S2 / n_rad_s * n_rad_s).powf(1.0 / 3.0);

        TleElements {
            name: tle.object_name.clone(),
            epoch: epoch.into(),
            a_km: a,
            e: *e_deg,
            i_rad: i_rad,
            raan_rad: raan_rad,
            argp_rad: argp_rad,
            m0_rad: m0_rad,
            n_rad_s: n_rad_s
        }
    }
}

#[derive(Debug)]
pub struct Point {
    pub time: DateTime<Utc>,
    pub eci_x_km: f64,
    pub eci_y_km: f64,
    pub eci_z_km: f64,
    pub eci_lat_deg: f64,
    pub eci_lon_deg: f64,
    pub eci_alt_km: f64,
    pub speed_km_s: f64,
    pub lat_deg: f64,
    pub lon_deg: f64,
    pub alt_km: f64,
}