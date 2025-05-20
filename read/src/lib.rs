use diesel_migrations::{embed_migrations, EmbeddedMigrations};

extern crate diesel;

#[path = "postgres/schema.rs"]
pub mod schema;

pub mod steps;

pub mod config;
pub mod models;
pub mod postgres;
pub mod processor;
pub mod utils;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("./src/postgres/migrations");
