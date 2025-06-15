pub mod build;
pub mod check;
mod logger;

pub use self::build::handle_build_command;
pub use self::check::handle_check_command;
pub use self::logger::{print_info, print_status, print_error, print_warning}; 