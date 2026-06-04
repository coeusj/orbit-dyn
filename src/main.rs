use tle::errors;

pub mod tle;

fn main() -> Result<(), errors::TleError>{
    let tle_str = tle::get_tle_str()?;
    let json = tle::parse_tle(tle_str)?;
    println!("{:#?}", json[0]);

    Ok(())
}
