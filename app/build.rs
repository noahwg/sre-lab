// Rebuild when a migration is added or changed: `sqlx::migrate!` embeds the files at
// compile time, and cargo can't see them on its own.
fn main() {
    println!("cargo:rerun-if-changed=migrations");
}
