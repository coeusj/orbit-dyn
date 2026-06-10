mod tle;

use chrono::{Utc, Duration};
use tle::errors;

fn main() -> Result<(), errors::TleError> {
    const STEP_SECONDS: i64 = 5;

    let sat_name = String::from("GSAT0201 (GALILEO 5)");
    let sat_tle = tle::get_sat_tle(sat_name)?;
    let start = Utc::now();
    let end = start + Duration::days(1);
    let points = tle::propgate(sat_tle, start, end, STEP_SECONDS);

    println!("GSAT0201 (GALILEO 5) - samples ({} points)", points.len());
    Ok(())
}
