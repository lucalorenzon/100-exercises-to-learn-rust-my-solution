// This is our last exercise. Let's go down a more unstructured path!
// Try writing an **asynchronous REST API** to expose the functionality
// of the ticket management system we built throughout the course.
// It should expose endpoints to:
//  - Create a ticket
//  - Retrieve ticket details
//  - Patch a ticket
//
// Use Rust's package registry, crates.io, to find the dependencies you need
// (if any) to build this system.

mod description;
pub mod service;
mod status;
pub mod ticket;
pub mod ticket_store;
mod title;
pub mod warp_handlers;
pub mod warp_routes;
pub mod warp_server;
