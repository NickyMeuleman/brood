fn main() {
    // sqlx: trigger a rebuild when a migration file changes or is added
    println!("cargo:rerun-if-changed=migrations");
    tauri_build::build()
}
