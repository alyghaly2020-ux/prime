//! Desktop automation module — mouse, keyboard, screen capture.
//!
//! Cross-platform via [`enigo`] for input simulation and [`xcap`] for screen
//! capture. Provides a high-level [`ComputerUseManager`] orchestrator as well
//! as standalone [`MouseController`], [`KeyboardController`], and
//! [`ScreenCapture`] subsystems.
//!
//! # Architecture
//!
//! ```text
//! ComputerUseManager
//!   ├── MouseController   (enigo::Enigo)
//!   ├── KeyboardController (enigo::Enigo)
//!   └── ScreenCapture     (xcap::Monitor)
//! ```
//!
//! # Errors
//!
//! All fallible operations return [`ComputerUseError`] with a descriptive
//! message. Callers can convert to [`AppError`](crate::AppError) via
//! `.map_err(|e| AppError::Execution(e.to_string()))`.

use enigo::{
    self, Button, Coordinate, Direction, Enigo, Key as EnigoKey, Keyboard, Mouse,
};
use image::codecs::png::PngEncoder;
use image::ImageEncoder;
use thiserror::Error;
use xcap::Monitor;

use std::sync::Mutex;

// =============================================================================
// Error Type
// =============================================================================

/// Errors that can occur during computer-use operations.
#[derive(Debug, Error)]
pub enum ComputerUseError {
    /// Screen capture failed (xcap or image encoding error).
    #[error("screen capture failed: {0}")]
    Capture(String),

    /// Input simulation failed (enigo error).
    #[error("input simulation failed: {0}")]
    Input(String),

    /// The requested display was not found.
    #[error("display not found: id={0}")]
    DisplayNotFound(u32),

    /// OCR is not available or text was not found on screen.
    #[error("ocr error: {0}")]
    Ocr(String),

    /// Image encoding or decoding failed.
    #[error("image encoding failed: {0}")]
    Encoding(String),
}

// Required for Tauri IPC (serializes as plain string).
impl serde::Serialize for ComputerUseError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

// =============================================================================
// Data Types
// =============================================================================

/// Information about a connected display / monitor.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DisplayInfo {
    /// Zero-based display identifier.
    pub id: u32,
    /// Platform display name (e.g. `"eDP-1"`, `"\\\\.\\DISPLAY1"`).
    pub name: String,
    /// Visible width in pixels.
    pub width: u32,
    /// Visible height in pixels.
    pub height: u32,
    /// X offset relative to the virtual desktop origin.
    pub x: i32,
    /// Y offset relative to the virtual desktop origin.
    pub y: i32,
    /// Whether this is the primary / main display.
    pub is_primary: bool,
}

/// Mouse buttons supported for click simulation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

/// Subset of virtual-key codes exposed for keyboard automation.
///
/// Maps directly to [`enigo::Key`] internally. Letter and number variants
/// produce the **lowercase** character; use [`KeyboardController::hotkey`] with
/// [`VirtualKey::Shift`] for uppercase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum VirtualKey {
    // -- Modifiers -----------------------------------------------------------
    Control,
    Shift,
    Alt,
    Meta,

    // -- Navigation ---------------------------------------------------------
    Up,
    Down,
    Left,
    Right,
    PageUp,
    PageDown,
    Home,
    End,

    // -- Editing ------------------------------------------------------------
    Backspace,
    Delete,
    Insert,
    Tab,
    Escape,
    Return,
    Space,

    // -- Function keys ------------------------------------------------------
    F1,  F2,  F3,  F4,  F5,  F6,
    F7,  F8,  F9,  F10, F11, F12,

    // -- Letters (lowercase) ------------------------------------------------
    A, B, C, D, E, F, G, H, I, J, K, L, M,
    N, O, P, Q, R, S, T, U, V, W, X, Y, Z,

    // -- Digits -------------------------------------------------------------
    Num0, Num1, Num2, Num3, Num4, Num5, Num6, Num7, Num8, Num9,

    // -- Symbols ------------------------------------------------------------
    Minus,
    Equals,
    BracketLeft,
    BracketRight,
    Semicolon,
    Quote,
    Comma,
    Period,
    Slash,
    Backslash,
    Backquote,

    // -- Misc ---------------------------------------------------------------
    CapsLock,
    PrintScreen,
    ScrollLock,
    Pause,
    Menu,
}

// =============================================================================
// Internal Helpers
// =============================================================================

/// Encode an [`image::RgbaImage`] to in-memory PNG bytes.
fn encode_png(img: &image::RgbaImage) -> Result<Vec<u8>, ComputerUseError> {
    let mut buf = Vec::new();
    let encoder = PngEncoder::new(&mut buf);
    encoder
        .write_image(img.as_raw(), img.width(), img.height(), image::ExtendedColorType::Rgba8)
        .map_err(|e| ComputerUseError::Encoding(e.to_string()))?;
    Ok(buf)
}

/// Map a [`VirtualKey`] to the corresponding [`enigo::Key`].
fn map_virtual_key(key: VirtualKey) -> EnigoKey {
    use VirtualKey::*;
    match key {
        // Modifiers
        Control => EnigoKey::Control,
        Shift => EnigoKey::Shift,
        Alt => EnigoKey::Alt,
        Meta => EnigoKey::Meta,

        // Navigation
        Up => EnigoKey::UpArrow,
        Down => EnigoKey::DownArrow,
        Left => EnigoKey::LeftArrow,
        Right => EnigoKey::RightArrow,
        PageUp => EnigoKey::PageUp,
        PageDown => EnigoKey::PageDown,
        Home => EnigoKey::Home,
        End => EnigoKey::End,

        // Editing
        Backspace => EnigoKey::Backspace,
        Delete => EnigoKey::Delete,
        Insert => EnigoKey::Insert,
        Tab => EnigoKey::Tab,
        Escape => EnigoKey::Escape,
        Return => EnigoKey::Return,
        Space => EnigoKey::Space,

        // Function keys
        F1 => EnigoKey::F1,
        F2 => EnigoKey::F2,
        F3 => EnigoKey::F3,
        F4 => EnigoKey::F4,
        F5 => EnigoKey::F5,
        F6 => EnigoKey::F6,
        F7 => EnigoKey::F7,
        F8 => EnigoKey::F8,
        F9 => EnigoKey::F9,
        F10 => EnigoKey::F10,
        F11 => EnigoKey::F11,
        F12 => EnigoKey::F12,

        // Letters
        A => EnigoKey::Unicode('a'),
        B => EnigoKey::Unicode('b'),
        C => EnigoKey::Unicode('c'),
        D => EnigoKey::Unicode('d'),
        E => EnigoKey::Unicode('e'),
        F => EnigoKey::Unicode('f'),
        G => EnigoKey::Unicode('g'),
        H => EnigoKey::Unicode('h'),
        I => EnigoKey::Unicode('i'),
        J => EnigoKey::Unicode('j'),
        K => EnigoKey::Unicode('k'),
        L => EnigoKey::Unicode('l'),
        M => EnigoKey::Unicode('m'),
        N => EnigoKey::Unicode('n'),
        O => EnigoKey::Unicode('o'),
        P => EnigoKey::Unicode('p'),
        Q => EnigoKey::Unicode('q'),
        R => EnigoKey::Unicode('r'),
        S => EnigoKey::Unicode('s'),
        T => EnigoKey::Unicode('t'),
        U => EnigoKey::Unicode('u'),
        V => EnigoKey::Unicode('v'),
        W => EnigoKey::Unicode('w'),
        X => EnigoKey::Unicode('x'),
        Y => EnigoKey::Unicode('y'),
        Z => EnigoKey::Unicode('z'),

        // Digits
        Num0 => EnigoKey::Unicode('0'),
        Num1 => EnigoKey::Unicode('1'),
        Num2 => EnigoKey::Unicode('2'),
        Num3 => EnigoKey::Unicode('3'),
        Num4 => EnigoKey::Unicode('4'),
        Num5 => EnigoKey::Unicode('5'),
        Num6 => EnigoKey::Unicode('6'),
        Num7 => EnigoKey::Unicode('7'),
        Num8 => EnigoKey::Unicode('8'),
        Num9 => EnigoKey::Unicode('9'),

        // Symbols
        Minus => EnigoKey::Unicode('-'),
        Equals => EnigoKey::Unicode('='),
        BracketLeft => EnigoKey::Unicode('['),
        BracketRight => EnigoKey::Unicode(']'),
        Semicolon => EnigoKey::Unicode(';'),
        Quote => EnigoKey::Unicode('\''),
        Comma => EnigoKey::Unicode(','),
        Period => EnigoKey::Unicode('.'),
        Slash => EnigoKey::Unicode('/'),
        Backslash => EnigoKey::Unicode('\\'),
        Backquote => EnigoKey::Unicode('`'),

        // Misc
        CapsLock => EnigoKey::CapsLock,
        PrintScreen => EnigoKey::Print,
        ScrollLock => EnigoKey::ScrollLock,
        Pause => EnigoKey::Pause,
        Menu => EnigoKey::LMenu,
    }
}

// =============================================================================
// ScreenCapture
// =============================================================================

/// Take screenshots of full displays or arbitrary regions.
///
/// Wraps [`xcap::Monitor`] and encodes captured frames as PNG byte buffers
/// suitable for sending over IPC or saving to disk.
///
/// # Errors
///
/// All methods return [`ComputerUseError::Capture`] when the underlying
/// platform API fails, and [`ComputerUseError::Encoding`] when PNG encoding
/// fails.
pub struct ScreenCapture;

impl ScreenCapture {
    /// Create a new [`ScreenCapture`] instance.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Capture the entire display identified by `display_id` as PNG bytes.
    ///
    /// `display_id` is a zero-based index into the list of connected monitors
    /// returned by [`get_display_info`](Self::get_display_info).
    ///
    /// # Errors
    ///
    /// Returns [`ComputerUseError::DisplayNotFound`] if the index exceeds the
    /// number of connected displays.
    pub fn capture_display(display_id: u32) -> Result<Vec<u8>, ComputerUseError> {
        let monitors = Monitor::all().map_err(|e| ComputerUseError::Capture(e.to_string()))?;
        let monitor = monitors
            .into_iter()
            .nth(display_id as usize)
            .ok_or(ComputerUseError::DisplayNotFound(display_id))?;
        let img = monitor
            .capture_image()
            .map_err(|e| ComputerUseError::Capture(e.to_string()))?;
        encode_png(&img)
    }

    /// Capture the **primary** display as PNG bytes.
    ///
    /// This is the display marked as primary by the OS (usually the one
    /// containing the taskbar / menu bar).
    ///
    /// # Errors
    ///
    /// Returns [`ComputerUseError::Capture`] if the primary display cannot be
    /// determined or the capture fails.
    pub fn capture_full_screenshot() -> Result<Vec<u8>, ComputerUseError> {
        let monitors = Monitor::all().map_err(|e| ComputerUseError::Capture(e.to_string()))?;
        let primary = monitors
            .into_iter()
            .find(|m| m.is_primary())
            .ok_or_else(|| ComputerUseError::Capture("no primary display found".into()))?;
        let img = primary
            .capture_image()
            .map_err(|e| ComputerUseError::Capture(e.to_string()))?;
        encode_png(&img)
    }

    /// Return metadata for every connected display.
    ///
    /// # Errors
    ///
    /// Returns [`ComputerUseError::Capture`] if the display list cannot be
    /// enumerated.
    pub fn get_display_info() -> Result<Vec<DisplayInfo>, ComputerUseError> {
        let monitors = Monitor::all().map_err(|e| ComputerUseError::Capture(e.to_string()))?;
        Ok(monitors
            .into_iter()
            .enumerate()
            .map(|(i, m)| DisplayInfo {
                id: i as u32,
                name: m.name().to_string(),
                width: m.width(),
                height: m.height(),
                x: m.x(),
                y: m.y(),
                is_primary: m.is_primary(),
            })
            .collect())
    }

    /// Capture a rectangular region of the screen as PNG bytes.
    ///
    /// Coordinates are absolute virtual-desktop coordinates. If the region
    /// spans multiple monitors, only the portion on the first intersecting
    /// monitor is returned. The returned image may be smaller than `(w × h)`
    /// if part of the region falls outside any display.
    ///
    /// # Errors
    ///
    /// Returns [`ComputerUseError::Capture`] if no display intersects the
    /// region or the capture fails.
    pub fn capture_region(x: i32, y: i32, w: u32, h: u32) -> Result<Vec<u8>, ComputerUseError> {
        let monitors = Monitor::all().map_err(|e| ComputerUseError::Capture(e.to_string()))?;
        for monitor in &monitors {
            let mx = monitor.x();
            let my = monitor.y();
            let mw = monitor.width() as i32;
            let mh = monitor.height() as i32;

            // Check axis-aligned overlap.
            if x + w as i32 > mx && x < mx + mw && y + h as i32 > my && y < my + mh {
                let mut img = monitor
                    .capture_image()
                    .map_err(|e| ComputerUseError::Capture(e.to_string()))?;

                // Clamp crop to monitor bounds.
                let crop_x = (x - mx).max(0) as u32;
                let crop_y = (y - my).max(0) as u32;
                let crop_w = w.min((mx + mw - x.max(mx)) as u32);
                let crop_h = h.min((my + mh - y.max(my)) as u32);

                let sub = image::imageops::crop(&mut img, crop_x, crop_y, crop_w, crop_h);
                return encode_png(&sub.to_image());
            }
        }
        Err(ComputerUseError::Capture(format!(
            "region ({x}, {y}) {w}×{h} is not within any display"
        )))
    }
}

impl Default for ScreenCapture {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// MouseController
// =============================================================================

/// Simulate mouse movements, clicks, scrolling, and dragging.
///
/// Wraps a single [`enigo::Enigo`] instance behind a [`Mutex`] to allow
/// shared access across threads.
///
/// # Errors
///
/// All methods return [`ComputerUseError::Input`] when the underlying
/// input-simulation backend fails.
pub struct MouseController {
    enigo: Mutex<Enigo>,
}

impl MouseController {
    /// Initialise the mouse controller.
    ///
    /// Opens the platform-specific input-simulation connection (X11 display,
    /// CGEvent source, or Windows input handle).
    ///
    /// # Errors
    ///
    /// Returns [`ComputerUseError::Input`] if the [`Enigo`] instance cannot
    /// be created (e.g. no display server available on Linux).
    pub fn new() -> Result<Self, ComputerUseError> {
        // Enigo::new() may fail on headless systems (no display server).
        let enigo = Enigo::new(&enigo::Settings::default())
            .map_err(|e| ComputerUseError::Input(e.to_string()))?;
        Ok(Self {
            enigo: Mutex::new(enigo),
        })
    }

    /// Move the mouse pointer to absolute screen coordinates `(x, y)`.
    pub fn move_to(&self, x: i32, y: i32) -> Result<(), ComputerUseError> {
        self.enigo
            .lock()
            .map_err(|e| ComputerUseError::Input(e.to_string()))?
            .move_mouse(x, y, Coordinate::Abs)
            .map_err(|e| ComputerUseError::Input(e.to_string()))
    }

    /// Perform a single click with the given [`MouseButton`].
    pub fn click(&self, button: MouseButton) -> Result<(), ComputerUseError> {
        let btn = match button {
            MouseButton::Left => Button::Left,
            MouseButton::Right => Button::Right,
            MouseButton::Middle => Button::Middle,
        };
        self.enigo
            .lock()
            .map_err(|e| ComputerUseError::Input(e.to_string()))?
            .button(btn, Direction::Click)
            .map_err(|e| ComputerUseError::Input(e.to_string()))
    }

    /// Perform two rapid left-clicks at the current cursor position.
    pub fn double_click(&self) -> Result<(), ComputerUseError> {
        let mut enigo = self
            .enigo
            .lock()
            .map_err(|e| ComputerUseError::Input(e.to_string()))?;
        enigo
            .button(Button::Left, Direction::Click)
            .map_err(|e| ComputerUseError::Input(e.to_string()))?;
        enigo
            .button(Button::Left, Direction::Click)
            .map_err(|e| ComputerUseError::Input(e.to_string()))?;
        Ok(())
    }

    /// Drag the mouse from `(start_x, start_y)` to `(end_x, end_y)` while
    /// holding the left button.
    pub fn drag(
        &self,
        start_x: i32,
        start_y: i32,
        end_x: i32,
        end_y: i32,
    ) -> Result<(), ComputerUseError> {
        let mut enigo = self
            .enigo
            .lock()
            .map_err(|e| ComputerUseError::Input(e.to_string()))?;
        enigo
            .move_mouse(start_x, start_y, Coordinate::Abs)
            .map_err(|e| ComputerUseError::Input(e.to_string()))?;
        enigo
            .button(Button::Left, Direction::Press)
            .map_err(|e| ComputerUseError::Input(e.to_string()))?;
        enigo
            .move_mouse(end_x, end_y, Coordinate::Abs)
            .map_err(|e| ComputerUseError::Input(e.to_string()))?;
        enigo
            .button(Button::Left, Direction::Release)
            .map_err(|e| ComputerUseError::Input(e.to_string()))?;
        Ok(())
    }

    /// Scroll horizontally (`delta_x`) and/or vertically (`delta_y`).
    ///
    /// Positive `delta_y` scrolls down / towards the user; negative scrolls
    /// up / away. `delta_x` scrolls horizontally on supporting platforms.
    pub fn scroll(&self, delta_x: i32, delta_y: i32) -> Result<(), ComputerUseError> {
        let mut enigo = self
            .enigo
            .lock()
            .map_err(|e| ComputerUseError::Input(e.to_string()))?;
        enigo
            .scroll(delta_y, enigo::Axis::Vertical)
            .map_err(|e| ComputerUseError::Input(e.to_string()))?;
        if delta_x != 0 {
            enigo
                .scroll(delta_x, enigo::Axis::Horizontal)
                .map_err(|e| ComputerUseError::Input(e.to_string()))?;
        }
        Ok(())
    }

    /// Return the current mouse cursor position in absolute screen
    /// coordinates.
    pub fn get_position(&self) -> Result<(i32, i32), ComputerUseError> {
        self.enigo
            .lock()
            .map_err(|e| ComputerUseError::Input(e.to_string()))?
            .location()
            .map_err(|e| ComputerUseError::Input(e.to_string()))
    }
}

// =============================================================================
// KeyboardController
// =============================================================================

/// Simulate keyboard input — individual keys, text typing, and hotkeys.
///
/// Wraps a single [`enigo::Enigo`] instance behind a [`Mutex`].
///
/// # Errors
///
/// All methods return [`ComputerUseError::Input`] when the underlying
/// input-simulation backend fails.
pub struct KeyboardController {
    enigo: Mutex<Enigo>,
}

impl KeyboardController {
    /// Initialise the keyboard controller.
    ///
    /// # Errors
    ///
    /// Returns [`ComputerUseError::Input`] if the [`Enigo`] instance cannot
    /// be created.
    pub fn new() -> Result<Self, ComputerUseError> {
        // Enigo::new() may fail on headless systems (no display server).
        let enigo = Enigo::new(&enigo::Settings::default())
            .map_err(|e| ComputerUseError::Input(e.to_string()))?;
        Ok(Self {
            enigo: Mutex::new(enigo),
        })
    }

    /// Press and release a single [`VirtualKey`].
    pub fn key_click(&self, key: VirtualKey) -> Result<(), ComputerUseError> {
        let k = map_virtual_key(key);
        self.enigo
            .lock()
            .map_err(|e| ComputerUseError::Input(e.to_string()))?
            .key(k, Direction::Click)
            .map_err(|e| ComputerUseError::Input(e.to_string()))
    }

    /// Type a string of text at the current cursor position.
    ///
    /// Uses the platform's text-composition mechanism (not individual key
    /// presses), so special characters and Unicode are handled correctly
    /// where supported.
    pub fn type_text(&self, text: &str) -> Result<(), ComputerUseError> {
        self.enigo
            .lock()
            .map_err(|e| ComputerUseError::Input(e.to_string()))?
            .text(text)
            .map_err(|e| ComputerUseError::Input(e.to_string()))
    }

    /// Press and hold a [`VirtualKey`] (without releasing).
    pub fn key_down(&self, key: VirtualKey) -> Result<(), ComputerUseError> {
        let k = map_virtual_key(key);
        self.enigo
            .lock()
            .map_err(|e| ComputerUseError::Input(e.to_string()))?
            .key(k, Direction::Press)
            .map_err(|e| ComputerUseError::Input(e.to_string()))
    }

    /// Release a previously-pressed [`VirtualKey`].
    pub fn key_up(&self, key: VirtualKey) -> Result<(), ComputerUseError> {
        let k = map_virtual_key(key);
        self.enigo
            .lock()
            .map_err(|e| ComputerUseError::Input(e.to_string()))?
            .key(k, Direction::Release)
            .map_err(|e| ComputerUseError::Input(e.to_string()))
    }

    /// Press multiple keys simultaneously, then release in reverse order.
    ///
    /// Used for keyboard shortcuts such as Ctrl+C, Alt+Tab, etc.
    ///
    /// # Example
    ///
    /// ```ignore
    /// controller.hotkey(&[VirtualKey::Control, VirtualKey::C])?; // copy
    /// controller.hotkey(&[VirtualKey::Alt, VirtualKey::Tab])?;   // switch window
    /// ```
    pub fn hotkey(&self, keys: &[VirtualKey]) -> Result<(), ComputerUseError> {
        let mut enigo = self
            .enigo
            .lock()
            .map_err(|e| ComputerUseError::Input(e.to_string()))?;

        // Press in order.
        for key in keys {
            enigo
                .key(map_virtual_key(*key), Direction::Press)
                .map_err(|e| ComputerUseError::Input(e.to_string()))?;
        }
        // Release in reverse order.
        for key in keys.iter().rev() {
            enigo
                .key(map_virtual_key(*key), Direction::Release)
                .map_err(|e| ComputerUseError::Input(e.to_string()))?;
        }
        Ok(())
    }
}

// =============================================================================
// ComputerUseManager  (top-level orchestrator)
// =============================================================================

/// High-level orchestrator combining mouse, keyboard, and screen-capture
/// subsystems into convenience operations for AI agents.
///
/// # Example
///
/// ```ignore
/// use computer_use::ComputerUseManager;
///
/// let cu = ComputerUseManager::new()?;
///
/// // Take a screenshot.
/// let png = cu.screenshot_and_analyze()?;
///
/// // Move to (100, 200) and type a message.
/// cu.type_at(100, 200, "Hello, world!")?;
///
/// // Press Ctrl+C.
/// cu.keyboard.hotkey(&[VirtualKey::Control, VirtualKey::C])?;
/// ```
pub struct ComputerUseManager {
    /// Mouse control subsystem.
    pub mouse: MouseController,
    /// Keyboard control subsystem.
    pub keyboard: KeyboardController,
    /// Screen capture subsystem.
    pub screen: ScreenCapture,
}

impl ComputerUseManager {
    /// Create a new [`ComputerUseManager`] with all subsystems initialised.
    ///
    /// # Errors
    ///
    /// Returns [`ComputerUseError::Input`] if either the mouse or keyboard
    /// controller cannot be initialised.
    pub fn new() -> Result<Self, ComputerUseError> {
        Ok(Self {
            mouse: MouseController::new()?,
            keyboard: KeyboardController::new()?,
            screen: ScreenCapture::new(),
        })
    }

    /// Move the mouse to `(x, y)` and perform a left-click.
    pub fn click_at(&self, x: i32, y: i32) -> Result<(), ComputerUseError> {
        self.mouse.move_to(x, y)?;
        self.mouse.click(MouseButton::Left)?;
        Ok(())
    }

    /// Move to `(x, y)`, click to focus, then type `text`.
    pub fn type_at(&self, x: i32, y: i32, text: &str) -> Result<(), ComputerUseError> {
        self.click_at(x, y)?;
        self.keyboard.type_text(text)?;
        Ok(())
    }

    /// Capture the primary display and return PNG bytes for analysis by an
    /// AI vision model or OCR engine.
    pub fn screenshot_and_analyze(&self) -> Result<Vec<u8>, ComputerUseError> {
        ScreenCapture::capture_full_screenshot()
    }

    /// Search for `text` on screen using OCR, then click on it.
    ///
    /// **Note:** A full implementation requires OCR with positional / bounding-
    /// box output (e.g. Tesseract with bounding boxes). The current
    /// implementation logs the search and returns `false`. Enable the
    /// `tesseract` feature and integrate with [`crate::browser::ocr::OcrEngine`]
    /// for actual text detection.
    #[must_use]
    pub fn find_and_click_text(&self, text: &str) -> bool {
        tracing::warn!(
            "find_and_click_text requires OCR with bounding-box support; \
             searched for '{text}' but no coordinates are available"
        );
        false
    }

    /// Return `(width, height)` of the primary display in pixels.
    ///
    /// # Errors
    ///
    /// Returns [`ComputerUseError::Capture`] if the display list cannot be
    /// enumerated or no primary display exists.
    pub fn get_screen_size(&self) -> Result<(u32, u32), ComputerUseError> {
        let primary = Monitor::all()
            .map_err(|e| ComputerUseError::Capture(e.to_string()))?
            .into_iter()
            .find(|m| m.is_primary())
            .ok_or_else(|| ComputerUseError::Capture("no primary display found".into()))?;
        Ok((primary.width(), primary.height()))
    }

    /// Provide visual feedback by highlighting a rectangular region on screen.
    ///
    /// The current implementation logs the request. A future version should
    /// create a transient, frameless, always-on-top window with a coloured
    /// border at `(x, y, w, h)` using platform-specific windowing APIs.
    pub fn highlight_region(&self, x: i32, y: i32, w: u32, h: u32) {
        tracing::info!(
            "highlight region requested: ({x}, {y}) {w}×{h} \
             — platform-specific overlay not yet implemented"
        );
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -- Error tests ---------------------------------------------------------

    #[test]
    fn test_error_display() {
        let err = ComputerUseError::Capture("x11 error".into());
        assert_eq!(err.to_string(), "screen capture failed: x11 error");

        let err = ComputerUseError::Input("enigo error".into());
        assert_eq!(err.to_string(), "input simulation failed: enigo error");

        let err = ComputerUseError::DisplayNotFound(99);
        assert_eq!(err.to_string(), "display not found: id=99");
    }

    #[test]
    fn test_error_serialize() {
        let err = ComputerUseError::Input("timeout".into());
        let json = serde_json::to_string(&err).unwrap();
        assert_eq!(json, "\"input simulation failed: timeout\"");
    }

    // -- Data-type tests ----------------------------------------------------

    #[test]
    fn test_display_info_serde() {
        let info = DisplayInfo {
            id: 0,
            name: "eDP-1".into(),
            width: 1920,
            height: 1080,
            x: 0,
            y: 0,
            is_primary: true,
        };
        let json = serde_json::to_string(&info).unwrap();
        let restored: DisplayInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(info.id, restored.id);
        assert_eq!(info.name, restored.name);
        assert_eq!(info.width, restored.width);
    }

    // -- VirtualKey mapping -------------------------------------------------

    #[test]
    fn test_virtual_key_mapping() {
        assert_eq!(map_virtual_key(VirtualKey::Control), EnigoKey::Control);
        assert_eq!(map_virtual_key(VirtualKey::Return), EnigoKey::Return);
        assert_eq!(map_virtual_key(VirtualKey::Escape), EnigoKey::Escape);
    }

    // -- ScreenCapture (no actual capture in unit tests) --------------------

    #[test]
    fn test_screen_capture_new() {
        let sc = ScreenCapture::new();
        // Structural validity — actual capture requires a display server.
        let _ = sc;
    }

    // -- MouseButton serde --------------------------------------------------

    #[test]
    fn test_mouse_button_serde() {
        for (btn, expected) in &[
            (MouseButton::Left, "\"Left\""),
            (MouseButton::Right, "\"Right\""),
            (MouseButton::Middle, "\"Middle\""),
        ] {
            assert_eq!(serde_json::to_string(btn).unwrap(), *expected);
        }
    }

    // -- VirtualKey serde ---------------------------------------------------

    #[test]
    fn test_virtual_key_serde() {
        let json = serde_json::to_string(&VirtualKey::Control).unwrap();
        assert_eq!(json, "\"Control\"");
        let restored: VirtualKey = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, VirtualKey::Control);
    }
}
