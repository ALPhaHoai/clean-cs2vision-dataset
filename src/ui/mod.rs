pub mod panels;
pub mod keyboard;

// Re-export commonly used functions
pub use panels::{
    render_top_panel,
    render_bottom_panel,
    render_label_panel,
    render_central_panel,
    render_delete_confirmation,
};

pub use keyboard::handle_keyboard_shortcuts;
