mod venues;
pub(crate) mod common;
pub (crate) mod error;
use std::sync::Arc;
use crate::venues::service::VenueService;

/// What every handler can receive with `State<AppState>`: the services of the catalogue.
#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) venues: Arc<VenueService>,
}


fn main() {
    println!("Hello, world!");
}
