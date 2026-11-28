use serde::{Deserialize, Serialize};
use  crate::error::FieldCode;



#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct VenueInput{
    name: Option<String>,
    city: Option<String>,
    time_zone: Option<String>,
    sectors: Option<Vec<SectorInput>>
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct SectorInput{
    name: Option<String>,
    seats: Option<Vec<SeatInput>>
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct SeatInput{
    row: Option<String>,
    number: Option<i64>
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VenueDto{
    id: String,
    name: String,
    city: String,
    time_zone:String,
    sectors: Vec<SectorDto>
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SectorDto{
    id: String,
    name: String,
    seats: Vec<SeatDto>
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SeatDto{
    row: String,
    number: i32
}