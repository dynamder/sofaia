//! The application's visual language.
//!
//! The *machinery* lives in [`ericed::style`]: the [`StyleExt`] trait, the
//! recipe traits, and the [`ericed::style::Tokens`] palette. What lives here is
//! the vocabulary — which kinds exist and what each one looks like — because
//! "a sidebar" and "a preview" only mean something in a file manager.
//!
//! Re-exporting [`StyleExt`] is deliberate: a view imports one module
//! (`crate::style`) and gets both the trait and the kinds it is applied to.

pub mod button;
pub mod container;
pub mod rule;
pub mod text;

pub use ericed::style::StyleExt;
