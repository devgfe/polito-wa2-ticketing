#![allow(dead_code)]

use chrono_tz::Tz;
use std::{collections::HashSet};
use crate::common::Violation;


#[derive(Debug, Clone, PartialEq, Eq, Copy, Hash)]
pub struct VenueId(i64);

impl VenueId {
    pub fn new(num: i64) -> Self {
        VenueId(num)
    }

    pub fn get(self) -> i64 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Copy, Hash)]
pub struct SectorId(i64);

impl SectorId {
    pub fn new(num: i64) -> Self {
        SectorId(num)
    }

    pub fn get(self) -> i64 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Venue {
    pub id: VenueId,
    pub name: VenueName,
    pub city: City,
    pub time_zone: Tz,
    pub owner_id: uuid::Uuid,
    pub sectors: Vec<Sector>,
}

impl Venue {
    pub fn new(
        id: VenueId,
        name: VenueName,
        city: City,
        time_zone: Tz,
        owner_id: uuid::Uuid,
        sectors: Vec<Sector>,
    ) -> Self {
        Venue {
            id,
            name,
            city,
            time_zone,
            owner_id,
            sectors,
        }
    }

    pub fn summary(self)-> VenueSummary{
        VenueSummary{
            id: self.id,
            name: self.name,
            city: self.city,
            time_zone: self.time_zone
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VenueSummary {
    pub id: VenueId,
    pub name: VenueName,
    pub city: City,
    pub time_zone: Tz,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sector {
    pub id: SectorId,
    pub name: SectorName,
    pub seats: Vec<Seat>,
}

impl Sector {
    pub fn new(id: SectorId, name: SectorName, seats: Vec<Seat>) -> Self {
        Sector { id, name, seats }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewVenue {
    name: VenueName,
    city: City,
    time_zone: Tz,
    owner_id: uuid::Uuid,
    sectors: Vec<NewSector>,
}

impl NewVenue {
    pub fn new(
        name: VenueName,
        city: City,
        time_zone: Tz,
        owner_id: uuid::Uuid,
        sectors: Vec<NewSector>,
    ) -> Result<Self, Violation> {
        if sectors.is_empty() {
            return Err(Violation::TooFewItems);
        }

        Ok(NewVenue {
            name,
            city,
            time_zone,
            owner_id,
            sectors,
        })
    }
    pub fn get_name(&self) -> &VenueName {
        &self.name
    }
    pub fn get_city(&self) -> &City {
        &self.city
    }
    pub fn get_time_zone(&self) -> Tz {
        self.time_zone
    }
    pub fn get_owner_id(&self) -> uuid::Uuid {
        self.owner_id
    }
    pub fn get_sectors(&self) -> &[NewSector] {
        &self.sectors
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewSector {
    name: SectorName,
    seats: Vec<Seat>,
}

impl NewSector {
    pub fn new(name: SectorName, seats: Vec<Seat>) -> Result<Self, Violation> {
        if seats.is_empty() {
            return Err(Violation::TooFewItems);
        }

        let mut seen = HashSet::with_capacity(seats.len());
        let duplicates = !seats.iter().all(|item| seen.insert(item));
        if duplicates {
            return Err(Violation::NotUnique);
        }

        Ok(NewSector { name, seats })
    }

    pub fn get_name(&self) -> &SectorName {
        &self.name
    }
    pub fn get_seats(&self) -> &[Seat] {
        &self.seats
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Seat {
    pub row: Row,
    pub number: SeatNumber,
}

impl Seat {
    pub fn new(row: Row, number: SeatNumber) -> Seat {
        Seat { row, number }
    }

    

}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VenueName(String);

impl VenueName {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for VenueName {
    type Error = Violation;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let chars = value.chars().count();
        if chars > 200 {
            return Err(Violation::TooLong);
        }
        if chars < 1 {
            return Err(Violation::TooShort);
        }
        Ok(VenueName(value))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectorName(String);

impl SectorName {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for SectorName {
    type Error = Violation;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let chars = value.chars().count();
        if chars > 100 {
            return Err(Violation::TooLong);
        }
        if chars < 1 {
            return Err(Violation::TooShort);
        }
        Ok(SectorName(value))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct City(String);

impl City {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for City {
    type Error = Violation;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let chars = value.chars().count();
        if chars > 100 {
            return Err(Violation::TooLong);
        }
        if chars < 1 {
            return Err(Violation::TooShort);
        }
        Ok(City(value))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Row(String);

impl Row {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for Row {
    type Error = Violation;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let chars = value.chars().count();
        if chars > 10 {
            return Err(Violation::TooLong);
        }
        if chars < 1 {
            return Err(Violation::TooShort);
        }
        Ok(Row(value))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
pub struct SeatNumber(i32);

impl SeatNumber {
    pub fn get(self) -> i32 {
        self.0
    }
}

impl TryFrom<i64> for SeatNumber{
    type Error = Violation;

    fn try_from(value: i64) -> Result<Self, Self::Error> {
        if value <= 0{
            return Err(Violation::OutOfRange)
        }
        if value > i32::MAX as i64{
            return Err(Violation::OutOfRange)
        }
        
        Ok(SeatNumber(value as i32))
    }
}

pub fn time_zone(value: &str) -> Result<Tz, Violation> {
    value.parse::<Tz>().map_err(|_| Violation::InvalidFormat)
}

// ------------------------- DOMAIN TESTS ------------------------

/// The values that the types accept and refuse, and the rules of a venue as a whole.
/// No service, no repository: only the types.
#[cfg(test)]
mod tests {
    use super::*;

    fn seat(row: &str, number: i64) -> Seat {
        Seat::new(
            Row::try_from(row.to_string()).unwrap(),
            SeatNumber::try_from(number).unwrap(),
        )
    }

    fn sector(seats: Vec<Seat>) -> Result<NewSector, Violation> {
        NewSector::new(SectorName::try_from("Stalls".to_string()).unwrap(), seats)
    }

    fn venue(sectors: Vec<NewSector>) -> Result<NewVenue, Violation> {
        NewVenue::new(
            VenueName::try_from("Teatro Regio".to_string()).unwrap(),
            City::try_from("Torino".to_string()).unwrap(),
            chrono_tz::Europe::Rome,
            uuid::Uuid::nil(),
            sectors,
        )
    }

    // Texts

    #[test]
    fn a_text_at_its_limits_is_accepted() {
        assert_eq!(Row::try_from("E".to_string()).unwrap().as_str(), "E");
        // 10 characters but 20 bytes: the limit counts characters.
        assert!(Row::try_from("è".repeat(10)).is_ok());
        assert!(City::try_from("a".repeat(100)).is_ok());
        assert!(SectorName::try_from("a".repeat(100)).is_ok());
        assert!(VenueName::try_from("a".repeat(200)).is_ok());
    }

    #[test]
    fn an_empty_text_is_too_short() {
        assert_eq!(Row::try_from(String::new()), Err(Violation::TooShort));
        assert_eq!(City::try_from(String::new()), Err(Violation::TooShort));
        assert_eq!(SectorName::try_from(String::new()), Err(Violation::TooShort));
        assert_eq!(VenueName::try_from(String::new()), Err(Violation::TooShort));
    }

    #[test]
    fn a_text_one_past_its_limit_is_too_long() {
        assert_eq!(Row::try_from("a".repeat(11)), Err(Violation::TooLong));
        assert_eq!(City::try_from("a".repeat(101)), Err(Violation::TooLong));
        assert_eq!(SectorName::try_from("a".repeat(101)), Err(Violation::TooLong));
        assert_eq!(VenueName::try_from("a".repeat(201)), Err(Violation::TooLong));
    }

    // Seat numbers and time zones

    #[test]
    fn a_seat_number_goes_from_1_to_the_largest_i32() {
        for number in [1, i32::MAX as i64] {
            assert_eq!(SeatNumber::try_from(number).unwrap().get() as i64, number);
        }
    }

    #[test]
    fn a_seat_number_out_of_range_is_refused() {
        for number in [0, -1, i32::MAX as i64 + 1] {
            assert_eq!(
                SeatNumber::try_from(number),
                Err(Violation::OutOfRange),
                "SeatNumber::try_from({number})"
            );
        }
    }

    #[test]
    fn a_time_zone_is_an_iana_name() {
        assert_eq!(time_zone("Europe/Rome"), Ok(chrono_tz::Europe::Rome));
        assert_eq!(time_zone("UTC"), Ok(Tz::UTC));
    }

    #[test]
    fn a_time_zone_of_another_form_is_refused() {
        for name in ["", "Roma", "europe/rome", "Europe/Rome "] {
            assert_eq!(time_zone(name), Err(Violation::InvalidFormat), "time_zone({name:?})");
        }
    }

    // Sectors

    #[test]
    fn a_sector_needs_at_least_one_seat() {
        assert_eq!(sector(vec![]), Err(Violation::TooFewItems));
    }

    #[test]
    fn a_sector_cannot_hold_the_same_seat_twice() {
        let seats = vec![seat("E", 14), seat("E", 15), seat("E", 14)];
        assert_eq!(sector(seats), Err(Violation::NotUnique));
    }

    #[test]
    fn seats_that_differ_in_row_or_number_are_different_seats() {
        let seats = vec![seat("E", 14), seat("E", 15), seat("F", 14)];
        assert!(sector(seats).is_ok());
    }

    // Venues

    #[test]
    fn a_venue_needs_at_least_one_sector() {
        assert_eq!(venue(vec![]), Err(Violation::TooFewItems));
        assert!(venue(vec![sector(vec![seat("A", 1)]).unwrap()]).is_ok());
    }
}