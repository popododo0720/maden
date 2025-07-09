use maden_macros::application;

mod handlers;

#[application(handlers::test_handler::TestHandler)]
async fn main() {
    // This function body will be replaced by the #[application] macro
}