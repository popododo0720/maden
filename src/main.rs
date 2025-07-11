use maden_macros::application;

mod handlers;

#[application]
async fn main() {
    // Logging initialization is now handled by maden-log crate via the #[application] macro
}
