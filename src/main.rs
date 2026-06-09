use chrono::Duration;
use tle::errors;

pub mod tle;

fn main() -> Result<(), errors::TleError> {
    const STEP_SECONDS: i64 = 5;

    let tle_str = tle::get_tle_str()?;
    let json = tle::parse_tle(tle_str)?;
    let sat_tle = &json[0];
    let start = tle::analysis_anchor_datetime()?;
    let end = start + Duration::days(1);
    let points = tle::propgate(sat_tle, start, end, STEP_SECONDS);

    println!("{} - samples ({} points)", sat_tle.object_name, points.len());

    Ok(())
}
