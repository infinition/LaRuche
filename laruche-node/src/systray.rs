//! Windows system tray icon for LaRuche.
//!
//! Shows a yellow hexagon icon in the notification area with:
//! - Double-click: open dashboard in browser
//! - Right-click menu: "Ouvrir le Dashboard", "Quitter"

#[cfg(windows)]
pub fn run_systray(port: u16, shutdown_tx: tokio::sync::oneshot::Sender<()>) {
    use tray_icon::{
        TrayIconBuilder, Icon,
        menu::{Menu, MenuItem, MenuEvent},
    };

    // Generate a 32x32 yellow hexagon RGBA icon
    let icon = generate_hex_icon(32, 32);
    let icon = Icon::from_rgba(icon, 32, 32).expect("Failed to create tray icon");

    // Build menu
    let menu = Menu::new();
    let item_open = MenuItem::new("Ouvrir le Dashboard", true, None);
    let item_quit = MenuItem::new("Quitter LaRuche", true, None);
    let _ = menu.append(&item_open);
    let _ = menu.append(&item_quit);

    let open_id = item_open.id().clone();
    let quit_id = item_quit.id().clone();
    let dashboard_url = format!("http://localhost:{}", port);

    let _tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip(&format!("LaRuche - localhost:{}", port))
        .with_icon(icon)
        .build()
        .expect("Failed to build tray icon");

    // Win32 event loop (required for tray icon on Windows)
    // This runs on its own thread, spawned from main
    let menu_rx = MenuEvent::receiver();

    loop {
        // Process win32 messages
        unsafe {
            let mut msg: winapi_msg = std::mem::zeroed();
            // PeekMessage with PM_REMOVE (non-blocking)
            while peek_message(&mut msg) {
                translate_message(&msg);
                dispatch_message(&msg);
            }
        }

        // Check menu events (non-blocking)
        if let Ok(event) = menu_rx.try_recv() {
            if event.id() == &open_id {
                let _ = std::process::Command::new("cmd").args(["/C", "start", &dashboard_url]).spawn();
            } else if event.id() == &quit_id {
                let _ = shutdown_tx.send(());
                break;
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}

#[cfg(not(windows))]
pub fn run_systray(_port: u16, _shutdown_tx: tokio::sync::oneshot::Sender<()>) {
    // No-op on non-Windows
}

/// Generate a yellow hexagon on transparent background as RGBA bytes.
fn generate_hex_icon(width: u32, height: u32) -> Vec<u8> {
    let mut rgba = vec![0u8; (width * height * 4) as usize];
    let cx = width as f64 / 2.0;
    let cy = height as f64 / 2.0;
    let r = (width.min(height) as f64 / 2.0) - 1.0;

    for y in 0..height {
        for x in 0..width {
            let dx = x as f64 - cx;
            let dy = y as f64 - cy;
            if is_inside_hexagon(dx, dy, r) {
                let idx = ((y * width + x) * 4) as usize;
                // Amber: #f59e0b
                rgba[idx] = 245;     // R
                rgba[idx + 1] = 158; // G
                rgba[idx + 2] = 11;  // B
                rgba[idx + 3] = 255; // A
            }
        }
    }
    rgba
}

/// Check if point (dx, dy) relative to center is inside a flat-top hexagon of radius r.
fn is_inside_hexagon(dx: f64, dy: f64, r: f64) -> bool {
    let ax = dx.abs();
    let ay = dy.abs();
    // Flat-top hexagon: pointy sides left/right
    // For a flat-top hex with circumradius r:
    // |y| <= r * sqrt(3)/2
    // |y| + |x| * sqrt(3) <= r * sqrt(3)
    let s3 = 3.0_f64.sqrt();
    ay <= r * s3 / 2.0 && ay + ax * s3 <= r * s3
}

// ─── Minimal Win32 message pump (no extra dependency) ────────────────────────

#[cfg(windows)]
#[repr(C)]
struct winapi_msg {
    hwnd: *mut std::ffi::c_void,
    message: u32,
    wparam: usize,
    lparam: isize,
    time: u32,
    pt_x: i32,
    pt_y: i32,
}

#[cfg(windows)]
extern "system" {
    fn PeekMessageW(msg: *mut winapi_msg, hwnd: *mut std::ffi::c_void, min: u32, max: u32, remove: u32) -> i32;
    fn TranslateMessage(msg: *const winapi_msg) -> i32;
    fn DispatchMessageW(msg: *const winapi_msg) -> isize;
}

#[cfg(windows)]
unsafe fn peek_message(msg: &mut winapi_msg) -> bool {
    PeekMessageW(msg, std::ptr::null_mut(), 0, 0, 1) != 0 // PM_REMOVE = 1
}

#[cfg(windows)]
unsafe fn translate_message(msg: &winapi_msg) {
    TranslateMessage(msg);
}

#[cfg(windows)]
unsafe fn dispatch_message(msg: &winapi_msg) {
    DispatchMessageW(msg);
}
