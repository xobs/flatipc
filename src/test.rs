mod mock;
use crate::MemoryMesage;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Default, flatipc_derive::IpcSafe)]
pub struct Point {
    pub x: i16,
    pub y: i16,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Default, flatipc_derive::IpcSafe)]
pub enum PixelColor {
    #[default]
    Dark,
    Light,
}

impl From<bool> for PixelColor {
    fn from(pc: bool) -> Self {
        if pc {
            PixelColor::Dark
        } else {
            PixelColor::Light
        }
    }
}

impl From<PixelColor> for bool {
    fn from(pc: PixelColor) -> bool {
        if pc == PixelColor::Dark {
            true
        } else {
            false
        }
    }
}

impl From<usize> for PixelColor {
    fn from(pc: usize) -> Self {
        if pc == 0 {
            PixelColor::Light
        } else {
            PixelColor::Dark
        }
    }
}

impl From<PixelColor> for usize {
    fn from(pc: PixelColor) -> usize {
        if pc == PixelColor::Light {
            0
        } else {
            1
        }
    }
}

/// Style properties for an object
#[derive(Debug, Copy, Clone, Default, flatipc_derive::IpcSafe)]
pub struct DrawStyle {
    /// Fill colour of the object
    pub fill_color: Option<PixelColor>,

    /// Stroke (border/line) color of the object
    pub stroke_color: Option<PixelColor>,

    /// Stroke width
    pub stroke_width: i16,
}

#[derive(Debug, Clone, Copy, Default, flatipc_derive::IpcSafe)]
pub struct Rectangle {
    /// Top left point of the rect
    pub tl: Point,

    /// Bottom right point of the rect
    pub br: Point,

    /// Drawing style
    pub style: DrawStyle,
}

/// coordinates are local to the canvas, not absolute to the screen
#[derive(Debug, Copy, Clone, flatipc_derive::IpcSafe)]
pub enum TextBounds {
    // fixed width and height in a rectangle
    BoundingBox(Rectangle),
    // fixed width, grows up from bottom right
    GrowableFromBr(Point, u16),
    // fixed width, grows down from top left
    GrowableFromTl(Point, u16),
    // fixed width, grows up from bottom left
    GrowableFromBl(Point, u16),
    // fixed width, grows down from top right
    GrowableFromTr(Point, u16),
    // fixed width, centered aligned top
    CenteredTop(Rectangle),
    // fixed width, centered aligned bottom
    CenteredBot(Rectangle),
}

impl Default for TextBounds {
    fn default() -> Self {
        TextBounds::BoundingBox(Rectangle::default())
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Default, flatipc_derive::IpcSafe)]
pub struct Gid {
    /// a 128-bit random identifier for graphical objects
    pub gid: [u32; 4],
}

#[derive(Debug, Copy, Clone, PartialEq, Default, flatipc_derive::IpcSafe)]
/// operations that may be requested of a TextView when sent to GAM
pub enum TextOp {
    #[default]
    Nop,
    Render,
    ComputeBounds, // maybe we don't need this
}

/// Style options for Latin script fonts
#[derive(Copy, Clone, Debug, PartialEq, Default, flatipc_derive::IpcSafe)]
pub enum GlyphStyle {
    #[default]
    Small = 0,
    Regular = 1,
    Bold = 2,
    Monospace = 3,
    Cjk = 4,
    Large = 5,
    ExtraLarge = 6,
    Tall = 7,
}

/// Point specifies a pixel coordinate
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Default, flatipc_derive::IpcSafe)]
pub struct Pt {
    pub x: i16,
    pub y: i16,
}

#[derive(Copy, Clone, Debug, PartialEq, Default, flatipc_derive::IpcSafe)]
pub struct Cursor {
    pub pt: Pt,
    pub line_height: usize,
}

#[derive(Clone, Debug, Default, flatipc_derive::Ipc)]
#[repr(C)]
pub struct TextView {
    /// The operation as specified for the GAM. Note this is different from the "op" when sent to
    /// graphics-server only the GAM should be sending TextViews to the graphics-server, and a different
    /// coding scheme is used for that link.
    operation: TextOp,

    /// GID of the canvas to draw on
    canvas: Gid,

    /// Set by the GAM to the canvas' clip_rect; needed by gfx
    /// for drawing. Note this is in screen coordinates.
    pub clip_rect: Option<Rectangle>,

    /// Render content with random stipples to indicate the strings within are untrusted
    pub untrusted: bool,

    /// optional 128-bit token which is presented to prove a field's trustability
    pub token: Option<[u32; 4]>,

    /// Only trusted, token-validated TextViews will have the invert bit respected
    pub invert: bool,

    /// offsets for text drawing -- exactly one of the following options should be specified
    /// note that the TextBounds coordinate system is local to the canvas, not the screen
    pub bounds_hint: TextBounds,

    /// Some(Rectangle) if bounds have been computed and text
    /// has not been modified. This is local to the canvas.
    pub bounds_computed: Option<Rectangle>,

    /// indicates if the text has overflowed the canvas, set by the drawing routine
    pub overflow: Option<bool>,
    /// callers should not set; use TexOp to select. gam-side bookkeepping, set to true if
    /// no drawing is desired and we just want to compute the bounds
    dry_run: bool,

    pub style: GlyphStyle,
    pub cursor: Cursor,
    /// this is the insertion point offset, if it's to be drawn, on the string
    pub insertion: Option<i32>,
    pub ellipsis: bool,

    pub draw_border: bool,
    /// you almost always want this to be true
    pub clear_area: bool,
    pub border_width: u16,
    /// radius of the rounded border, if applicable
    pub rounded_border: Option<u16>,
    pub margin: Point,

    /// this field specifies the beginning and end of a "selected" region of text
    pub selected: Option<[u32; 2]>,

    /// this field tracks the state of a busy animation, if `Some`
    pub busy_animation_state: Option<u32>,
    pub text: crate::String<3000>,
}

use crate::ToMemoryMessage;
#[test]
fn textview_general_test() {
    // Create a TextView with the default settings
    let tv = TextView::default();

    // Turn it into a PaddedTextView which is suitable for IPC.
    let mut tv_msg = tv.into_message();

    // Perform a `lend`. We can do this because padded versions have
    // `.lend(&self, opcode: usize)` and `.lend_mut(&mut self, opcode: usize)`
    // methods on them.
    tv_msg.lend(0, 42);

    // The
    tv_msg.draw_border = true;
    use core::fmt::Write;
    write!(&mut tv_msg.text, "Hello from the server!").unwrap();

    let original_tv = tv_msg.into_original();
    println!("Original textview: {:?}", original_tv);
}

#[test]
fn simple_ipc() {
    #[derive(flatipc_derive::Ipc, Debug)]
    #[repr(C)]
    pub enum SimpleIpc {
        Single(u32),
    }

    impl Default for SimpleIpc {
        fn default() -> Self {
            SimpleIpc::Single(0)
        }
    }

    let simple_ipc = SimpleIpc::default();

    println!("Simple IPC: {:?}", simple_ipc);
}

#[test]
fn server_test() {
    #[derive(flatipc_derive::Ipc, Debug)]
    #[repr(C)]
    struct Incrementer {
        value: u32,
    }

    let inc = Incrementer { value: 42 };

    let adder_server = mock::Server::new(
        Box::new(|opcode, a, b, buffer| {
            let flattened = IpcIncrementer::from_buffer(buffer, a).unwrap();
            println!(
                "In adder server. Opcode: {}.  Current increment value: {} (a: {}, b: {})",
                opcode, flattened.value, a, b
            );
            (0, 0)
        }),
        Box::new(|opcode, a, b, buffer| {
            println!("LendMut opcode: {} (a: {}, b: {})", opcode, a, b);
            let flattened = IpcIncrementer::from_buffer_mut(buffer, a).unwrap();
            flattened.value += 1;
            (0, 0)
        }),
    );

    let adder_server_connection = mock::IPC_MACHINE.lock().unwrap().add_server(adder_server);
    let mut lendable_inc = inc.into_message();
    println!("Value before: {}", lendable_inc.value);
    lendable_inc.lend(adder_server_connection, 0);
    println!("Value after: {}", lendable_inc.value);

    // Mutably lend the value and make sure the server can change the original
    println!("Value before mut: {}", lendable_inc.value);
    lendable_inc.lend_mut(adder_server_connection, 0);
    println!("Value after mut: {}", lendable_inc.value);

    // Turn it back into the original value
    let original_inc = lendable_inc.into_original();
    println!("Original value: {}", original_inc.value);
}
