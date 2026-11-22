// Rebuilds the service when a file of migrations/ changes or is added: sqlx::migrate!() builds
// the migrations into the binary, and cargo would not notice a new file by itself.
fn main() {
    println!("cargo:rerun-if-changed=migrations");
}
