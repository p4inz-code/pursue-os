//! PURSUE OS — Investigation Desktop Shell & IPC Client.
//!
//! Provides the primary forensic graphical environment for PURSUE OS.
//! Built with `egui` and `eframe`, this shell interfaces strictly through
//! local domain IPC with the backend services (`pursue-runtime`, `pursue-terminal`,
//! `pursue-browser`, `pursue-case`), enforcing zero direct filesystem or database coupling.

#![deny(unsafe_code)]
#![warn(missing_docs)]

pub mod app;
pub mod client;
pub mod state;
pub mod views;

pub use app::PursueDesktopApp;
pub use client::{IpcClient, RouterClient};
pub use state::{DesktopState, DesktopTab};
