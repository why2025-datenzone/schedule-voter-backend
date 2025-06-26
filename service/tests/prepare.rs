// use ::entity::post;
// use sea_orm::*;

#[cfg(feature = "mock")]
pub fn prepare_mock_db() -> DatabaseConnection {
	MockDatabase::new(DatabaseBackend::Postgres).into_connection()
}
