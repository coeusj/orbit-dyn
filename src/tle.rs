use chrono::{DateTime, Datelike, Duration, Timelike, Utc};

use crate::tle::errors::TleError;
use crate::tle::models::Point;
use crate::tle::models::SatTle;
use crate::tle::models::TleElements;

pub mod errors;
pub mod models;

pub fn get_tle_str() -> Result<String, errors::TleError> {
    let mut res = ureq::get("http://celestrak.com/NORAD/elements/gp.php")
        .query("NAME", "GSAT0201 (GALILEO 5)")
        .query("FORMAT","json")
        .call()?;
    let res_str = res.body_mut().read_to_string()?;
    Ok(res_str)
}

pub fn parse_tle(tle_str: String) -> Result<Vec<SatTle>, TleError>{
    let tle_deser: Vec<SatTle> = serde_json::from_str(&tle_str)?;
    Ok(tle_deser)
}

pub fn analysis_anchor_datetime() -> Result<DateTime<Utc>, TleError> {
    let now = Utc::now();
    Ok(now)
}

pub fn propgate(tle: &models::SatTle, start: DateTime<Utc>, end: DateTime<Utc>, steps_sec: i64) -> Vec<Point> {
    let elements = TleElements::new(tle);

    let mut points: Vec<Point> = Vec::new();
    let mut t = start;
    while t <= end {
        // 1) Mean anomaly at time t
        let dt_sec = (t - elements.epoch).as_seconds_f64();
        let mut M = elements.m0_rad + elements.n_rad_s + dt_sec;
        M = M % (2.0 * std::f64::consts::PI);
        if M < 0.0 {
            M += 2.0 + std::f64::consts::PI;
        }

        // 2) Solve Kepler
        let E = kepler_solve(M, elements.e);
        let cosE = E.cos();

        // 3) Radius and true anomaly
        let r_km = elements.a_km * (1.0 - elements.e * cosE);
        let cos_v = (cosE - elements.e) / (1.0 - elements.e * cosE);
        let sin_v = ((1.0 - elements.e * elements.e).sqrt() * E.sin()) / (1.0 - elements.e * cosE);
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
        let speed_km_s = (models::MU_EARTH_KM3_S2 * (2.0 / r_km - 1.0 / elements.a_km)).sqrt();

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

fn kepler_solve(M: f64, e: f64) -> f64 {
    /*
    * Solve Kepler's equation M = E - e sin(E) for E.
    * Newton-Raphson method.
    */
    let max_iter = 20;
    let tol  = 1e-8;

    let mut E = M;
    for _ in 0..max_iter {
        let f = E - e * E.sin() - M;
        let fp = 1.0 - e * E.cos();
        if fp == 0.0 {
            break;
        }

        let dE = f / fp;
        let E_new = E - dE;
        if dE < tol {
            return E_new;
        }

        E = E_new;
    }

    E
}

fn ecef_to_geodetic(r_ecef_km: [f64; 3]) -> [f64; 3] {
    /*
     * Very simple spherical conversion: ECEF [km] → (lat, lon, alt_km).
     * Good enough for a workshop visualization.
     */
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

    let alt_km = r - models::EARTH_RADIUS_KM;
    let lat_deg = lat.to_degrees();
    let lon_deg = lon.to_degrees();
    [lat_deg, lon_deg, alt_km]
}

fn eci_to_ecef(r_eci_km: [f64; 3], dt: DateTime<Utc>) -> [f64; 3] {
    /*
     * Rotate position from ECI to ECEF using GMST.
     * r_eci_km: [x, y, z] in km
     */

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

// Simple ECI → ECEF → geodetic helpers
fn gmst_angle(dt: &DateTime<Utc>) -> f64 {
    // Approximate Greenwich Mean Sidereal Time angle [rad].
    // Good enough for visualization.

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
    let t = (jd - 2451545.0) / 36525.0;

    let gmst_sec_raw = 67310.54841
        + (876600.0 * 3600.0 + 8640184.812866) * t
        + 0.093104 * t.powi(2)
        - 6.2e-6 * t.powi(3);

    let gmst_sec = gmst_sec_raw % 86400.0;

    // Convert time seconds in radians
    let gmst_rad = (gmst_sec / 240.0) * std::f64::consts::PI / 180.0;

    gmst_rad
}