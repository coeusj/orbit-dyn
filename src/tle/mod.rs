use chrono::{DateTime, Datelike, Duration, Timelike, Utc};

pub mod errors;
pub mod models;
mod constants;

use crate::tle::errors::TleError;
use crate::tle::models::Point;
use crate::tle::models::TLE;
use crate::tle::models::KeplerianElements;

pub fn get_sat_tle(sat_name: String) -> Result<TLE, TleError> {
    let mut res_json = ureq::get("http://celestrak.com/NORAD/elements/gp.php")
        .query("NAME", sat_name)
        .query("FORMAT","json")
        .call()?;

    let json_str = res_json.body_mut().read_to_string()?;
    let tle_vec: Vec<TLE> = serde_json::from_str(&json_str)?;
    Ok(tle_vec[0].clone())
}

pub fn propgate(tle: models::TLE, start: DateTime<Utc>, end: DateTime<Utc>, steps_sec: i64) -> Vec<Point> {
    let elements = KeplerianElements::new(tle);

    let mut points: Vec<Point> = Vec::new();
    let mut t = start;

    while t <= end {
        // 1) Mean anomaly at time t
        let dt_sec = (t - elements.epoch).as_seconds_f64();
        let mut mean_anomaly = elements.m0_rad + elements.n_rad_s + dt_sec;
        mean_anomaly = mean_anomaly % (2.0 * std::f64::consts::PI);
        if mean_anomaly < 0.0 {
            mean_anomaly += 2.0 + std::f64::consts::PI;
        }

        // 2) Solve Kepler
        let eccentric_anomaly = kepler_solve(mean_anomaly, elements.e);
        let eccentric_anomaly_cos = eccentric_anomaly.cos();

        // 3) Radius and true anomaly
        let r_km = elements.a_km * (1.0 - elements.e * eccentric_anomaly_cos);
        let cos_v = (eccentric_anomaly_cos - elements.e) / (1.0 - elements.e * eccentric_anomaly_cos);
        let sin_v = ((1.0 - elements.e * elements.e).sqrt() * eccentric_anomaly.sin()) / (1.0 - elements.e * eccentric_anomaly_cos);
        let v = sin_v.atan2(cos_v);

        // 4) Position in ECI
        let u = elements.argp_rad + v;
        let cos_u = u.cos();
        let sin_u = u.sin();
        let cos_i = elements.i_rad.cos();
        let sin_i = elements.i_rad.sin();
        let cos_o = elements.raan_rad.cos();
        let sin_o = elements.raan_rad.sin();

        let x = r_km * (cos_o * cos_u - sin_o * sin_u * cos_i);
        let y = r_km * (sin_o * cos_u + cos_o * sin_u *cos_i);
        let z = r_km * (sin_u * sin_i);

        let r_eci: [f64; 3] = [x, y, z];

        // Speed magnitude from vis-viva (km/s)
        let speed_km_s = (constants::MU_EARTH_KM3_S2 * (2.0 / r_km - 1.0 / elements.a_km)).sqrt();

        // Inertial (ECI) spherical lat/lon (no Earth rotation)
        let geo_eci = ecef_to_geodetic(r_eci);
        let eci_lat_deg = geo_eci[0];
        let eci_lon_deg = geo_eci[1];
        let eci_alt_km = geo_eci[2];

        // 5) ECI → ECEF → geodetic
        let r_ecef = eci_to_ecef(r_eci, t);
        let geo_ecef = ecef_to_geodetic(r_ecef);
        let lat_deg = geo_ecef[0];
        let lon_deg = geo_ecef[1];
        let alt_km = geo_ecef[2];

        points.push(models::Point {
            time: t,
            eci_x_km: x,
            eci_y_km: y,
            eci_z_km: z,
            eci_lat_deg,
            eci_lon_deg,
            eci_alt_km,
            speed_km_s,
            lat_deg,
            lon_deg,
            alt_km
        });

        t += Duration::seconds(steps_sec);
    }

    points
}

/// Solves Kepler's equation: M = E - e * sin(E) for the Eccentric Anomaly (E).
/// Uses the iterative Newton-Raphson numerical method.
fn kepler_solve(mean_anomaly: f64, eccentricity: f64) -> f64 {
    let max_iterations = 20;
    let tolerance  = 1e-8;

    let mut eccentric_anomaly = mean_anomaly;

    for _ in 0..max_iterations {
        let function_value = eccentric_anomaly - eccentricity * eccentric_anomaly.sin() - mean_anomaly;
        let derivative_value = 1.0 - eccentricity * eccentric_anomaly.cos();
        if derivative_value == 0.0 {
            break;
        }

        let correction_step = function_value / derivative_value;
        let next_eccentric_anomaly = eccentric_anomaly - correction_step;
        if correction_step.abs() < tolerance {
            return next_eccentric_anomaly;
        }

        eccentric_anomaly = next_eccentric_anomaly;
    }

    eccentric_anomaly
}

/// Very simple spherical conversion: ECEF [km] → (lat, lon, alt_km).
/// Good enough for a workshop visualization.
fn ecef_to_geodetic(r_ecef_km: [f64; 3]) -> [f64; 3] {
    let x = r_ecef_km[0];
    let y = r_ecef_km[1];
    let z = r_ecef_km[2];

    let r_xy = x.hypot(y);
    let r = (x * x + y * y + z *z).sqrt();

    let lat;
    let mut lon = 0.0;
    if r_xy == 0.0 {
        lat = (std::f64::consts::PI / 2.0).copysign(z);
    } else {
        lat = z.atan2(r_xy);
        lon = y.atan2(x);
    }

    let alt_km = r - constants::EARTH_RADIUS_KM;
    let lat_deg = lat.to_degrees();
    let lon_deg = lon.to_degrees();
    [lat_deg, lon_deg, alt_km]
}

/// Rotate position from ECI to ECEF using GMST.
/// r_eci_km: [x, y, z] in km
fn eci_to_ecef(r_eci_km: [f64; 3], dt: DateTime<Utc>) -> [f64; 3] {
    let theta = gmst_angle(&dt);
    let cos_t = theta.cos();
    let sin_t = theta.sin();

    let x = r_eci_km[0];
    let y = r_eci_km[1];
    let z = r_eci_km[2];

    let x_ecef = cos_t * x + sin_t * y;
    let y_ecef = -(sin_t) * x + cos_t * y;
    let z_ecef = z;
    [x_ecef, y_ecef, z_ecef]
}

/// Simple ECI → ECEF → geodetic helpers
/// Approximate Greenwich Mean Sidereal Time angle [rad].
/// Good enough for visualization.
fn gmst_angle(dt: &DateTime<Utc>) -> f64 {
    let year = dt.year() as f64;
    let month = dt.month() as f64;
    let day = dt.day() as f64;
    let hour = dt.hour() as f64;
    let minute = dt.minute() as f64;
    let second = dt.second() as f64;

    // Julian Date (simple formula)
    let jd = 367.0 * year
        - ((7.0 * (year + ((month + 9.0) / 12.0).floor())) / 4.0).floor()
        + ((275.0 * month) / 9.0).floor()
        + day
        + 1721013.5
        + (hour + minute / 60.0 + second / 3600.0) / 24.0;

    // Julian Centuries since the J2000.0 era
    let t = (jd - constants::JD_J2000_EPOCH) /  constants::DAYS_PER_JULIAN_CENTURY;

    let gmst_sec_raw = 67310.54841
        + (876600.0 * 3600.0 + 8640184.812866) * t
        + 0.093104 * t.powi(2)
        - 6.2e-6 * t.powi(3);

    let gmst_sec = gmst_sec_raw % constants::SECONDS_PER_DAY;

    // Convert time seconds in radians
    let gmst_rad = (gmst_sec / constants::SECONDS_PER_DEGREE_ROTATION) * std::f64::consts::PI / 180.0;

    gmst_rad
}