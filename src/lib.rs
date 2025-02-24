// #[macro_use]
extern crate diesel;

#[path = "postgres/schema.rs"]
pub mod schema;

pub mod processors;

pub mod postgres;
