mod prepare;

use axum_example_service::{Mutation, Query};
use entity::post;
use prepare::prepare_mock_db;

#[tokio::test]
async fn main() {
    let db = &prepare_mock_db();
}
