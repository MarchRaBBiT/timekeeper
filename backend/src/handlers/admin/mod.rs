pub mod audit_logs;
pub mod attendance;
pub mod common;
pub mod export;
pub mod holidays;
pub mod requests;
pub mod users;

pub use audit_logs::*;
pub use attendance::*;
// common is internal helpers, usually not re-exported fully, but let's see if docs.rs needs anything from it.
// docs.rs needs structs. The structs are in their respective modules now.
// We should re-export everything from the new modules to maintain backward compatibility for `use crate::handlers::admin::*;` if used.
pub use export::*;
pub use holidays::*;
pub use requests::*;
pub use users::*;
