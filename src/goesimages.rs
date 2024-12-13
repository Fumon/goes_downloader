use chrono::Utc;
use url::Url;

pub enum Sat {
    GoesEast,
    GoesWest,
}

impl Sat {
    pub const fn url_frag(&self) -> &'static str {
        match self {
            Sat::GoesEast => "GOES16",
            Sat::GoesWest => "GOES18",
        }
    }
}


// From https://cdn.star.nesdis.noaa.gov/GOES16/ABI/FD/GEOCOLOR/20243350830_GOES16-ABI-FD-GEOCOLOR-1808x1808.jpg
const CDN_PREFIX: &'static str = "cdn.star.nesdis.noaa.gov";
pub fn construct_image_url(sat: &Sat, time: &chrono::DateTime<Utc>) -> Result<Url, Box<dyn std::error::Error>> {
    let datetime = time.format("%Y%j%H%M");
    Ok(Url::parse(&format!("https://{CDN_PREFIX}/{sat_url_frag}/ABI/FD/GEOCOLOR/{datetime}_{sat_url_frag}-ABI-FD-GEOCOLOR-1808x1808.jpg", sat_url_frag = sat.url_frag()))?)
}


#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    #[test]
    fn parse_success() -> Result<(), Box<dyn std::error::Error>> {
        let datetime = chrono::Utc.with_ymd_and_hms(2024, 11, 30, 8, 30, 00).unwrap();
        let result = construct_image_url(&Sat::GoesEast, &datetime)?;
        assert_eq!("https://cdn.star.nesdis.noaa.gov/GOES16/ABI/FD/GEOCOLOR/20243350830_GOES16-ABI-FD-GEOCOLOR-1808x1808.jpg", result.as_str());
        Ok(())
    }
}