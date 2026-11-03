//! Native loader/resource boundaries for the Linux proof. No musical state.
#[cfg(all(target_os = "linux", target_arch = "x86_64", target_env = "gnu"))]
mod linux;
#[cfg(all(target_os = "linux", target_arch = "x86_64", target_env = "gnu"))]
pub(crate) use linux::{resource_root, retain_code, scope, thread_id};
