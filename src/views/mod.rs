//! Two kinds of view live here. `auth` holds the JSON response shapes of
//! the `/api/auth` controller (Loco's convention; unmounted, app.rs).
//! `layout` and the page modules hold the Leptos components the page
//! controllers render into HTML.

pub mod account;
pub mod auth;
pub mod error;
pub mod forgot;
pub mod home;
pub mod layout;
pub mod login;
pub mod mail;
pub mod not_found;
pub mod register;
pub mod reset;
pub mod too_many_requests;
pub mod verify;
