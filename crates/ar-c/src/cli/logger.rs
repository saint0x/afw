use console;

/// Print status message with proper formatting
pub fn print_status(status: &str, message: &str) {
    println!("    {} {}", 
        console::style(status).bold().green(), 
        message
    );
}

/// Print error message with proper formatting
pub fn print_error(message: &str) {
    eprintln!("    {} {}", 
        console::style("error").bold().red(), 
        message
    );
}

/// Print warning message with proper formatting
pub fn print_warning(message: &str) {
    println!("    {} {}", 
        console::style("warning").bold().yellow(), 
        message
    );
}

/// Print info message with proper formatting
pub fn print_info(message: &str) {
    println!("    {} {}", 
        console::style("info").bold().blue(), 
        message
    );
} 