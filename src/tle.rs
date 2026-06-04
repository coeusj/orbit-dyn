use serde::Deserialize;

pub mod errors;

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

pub fn get_tle_str() -> Result<String, errors::TleError> {
    let mut res = ureq::get("http://celestrak.com/NORAD/elements/gp.php")
        .query("NAME", "GSAT0201 (GALILEO 5)")
        .query("FORMAT","json")
        .call()?;
    let res_str = res.body_mut().read_to_string()?;
    Ok(res_str)
}

pub fn parse_tle(tle_str: String) -> Result<Vec<SatTle>, errors::TleError>{
    let tle_deser: Vec<SatTle> = serde_json::from_str(&tle_str)?;
    Ok(tle_deser)
}