#[derive(Copy, Clone, Debug, Eq, PartialEq, Default)]
pub struct Point {
    pub x: i16,
    pub y: i16,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Default)]
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
#[derive(Debug, Copy, Clone, Default)]
pub struct DrawStyle {
    /// Fill colour of the object
    pub fill_color: Option<PixelColor>,

    /// Stroke (border/line) color of the object
    pub stroke_color: Option<PixelColor>,

    /// Stroke width
    pub stroke_width: i16,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Rectangle {
    /// Top left point of the rect
    pub tl: Point,

    /// Bottom right point of the rect
    pub br: Point,

    /// Drawing style
    pub style: DrawStyle,
}

/// coordinates are local to the canvas, not absolute to the screen
#[derive(Debug, Copy, Clone)]
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

#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub struct Gid {
    /// a 128-bit random identifier for graphical objects
    pub gid: [u32; 4],
}

#[derive(Debug, Copy, Clone, PartialEq, Default)]
// operations that may be requested of a TextView when sent to GAM
pub enum TextOp {
    #[default]
    Nop,
    Render,
    ComputeBounds, // maybe we don't need this
}

/// Style options for Latin script fonts
#[derive(Copy, Clone, Debug, PartialEq, Default)]
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
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Default)]
pub struct Pt {
    pub x: i16,
    pub y: i16,
}

#[derive(Copy, Clone, Debug, PartialEq, Default)]
pub struct Cursor {
    pub pt: Pt,
    pub line_height: usize,
}

#[derive(Clone, Debug, Default)]
#[repr(C)]
pub struct TextView {
    // this is the operation as specified for the GAM. Note this is different from the "op" when sent to
    // graphics-server only the GAM should be sending TextViews to the graphics-server, and a different
    // coding scheme is used for that link.
    operation: TextOp,
    canvas: Gid, // GID of the canvas to draw on
    pub clip_rect: Option<Rectangle>, /* this is set by the GAM to the canvas' clip_rect; needed by gfx
                                       * for drawing. Note this is in screen coordinates. */

    pub untrusted: bool, // render content with random stipples to indicate the strings within are untrusted
    pub token: Option<[u32; 4]>, // optional 128-bit token which is presented to prove a field's trustability
    pub invert: bool, // only trusted, token-validated TextViews will have the invert bit respected

    // offsets for text drawing -- exactly one of the following options should be specified
    // note that the TextBounds coordinate system is local to the canvas, not the screen
    pub bounds_hint: TextBounds,
    pub bounds_computed: Option<Rectangle>, /* is Some(Rectangle) if bounds have been computed and text
                                             * has not been modified. This is local to the canvas. */
    pub overflow: Option<bool>, /* indicates if the text has overflowed the canvas, set by the drawing
                                 * routine */
    dry_run: bool, /* callers should not set; use TexOp to select. gam-side bookkeepping, set to true if
                    * no drawing is desired and we just want to compute the bounds */

    pub style: GlyphStyle,
    pub cursor: Cursor,
    pub insertion: Option<i32>, // this is the insertion point offset, if it's to be drawn, on the string
    pub ellipsis: bool,

    pub draw_border: bool,
    pub clear_area: bool, // you almost always want this to be true
    pub border_width: u16,
    pub rounded_border: Option<u16>, // radius of the rounded border, if applicable
    pub margin: Point,

    // this field specifies the beginning and end of a "selected" region of text
    pub selected: Option<[u32; 2]>,

    // this field tracks the state of a busy animation, if `Some`
    pub busy_animation_state: Option<u32>,
    // pub text: String,
}

const PAGE_SIZE: usize = 4096;
const TV_SIZE: usize = core::mem::size_of::<TextView>();
// Round tv_size up to the next 4096-byte boundary
const PADDED_SIZE: usize = ((TV_SIZE + (PAGE_SIZE - 1)) & !(PAGE_SIZE - 1)) - TV_SIZE;
#[repr(C, align(4096))]
pub struct PaddedTextView {
    data: [u8; TV_SIZE],
    padding: [u8; PADDED_SIZE],
}

pub trait MemoryMesage {
    type Original;
    fn to_original(self) -> Self::Original;
}

pub trait ToMemoryMessage {
    type Padded;
    fn into_message(self) -> Self::Padded;
}

impl ToMemoryMessage for TextView {
    type Padded = PaddedTextView;
    fn into_message(self) -> PaddedTextView {
        let mut padded = PaddedTextView {
            data: [0; TV_SIZE],
            padding: [0; PADDED_SIZE],
        };
        unsafe {
            let tv_ptr = &mut padded.data as *mut [u8; TV_SIZE] as *mut TextView;
            core::ptr::write(tv_ptr, self);
        }
        padded
    }
}

impl MemoryMesage for PaddedTextView {
    type Original = TextView;
    fn to_original(self) -> Self::Original {
        let mut tv = TextView::default();
        unsafe {
            let tv_ptr = &mut tv as *mut TextView as *mut [u8; TV_SIZE];
            core::ptr::copy(&self.data, tv_ptr, 1);
        }
        tv
    }
}

impl core::ops::Deref for PaddedTextView {
    type Target = TextView;
    fn deref(&self) -> &Self::Target {
        unsafe {
            let tv_ptr = &self.data as *const [u8; TV_SIZE] as *const TextView;
            &*tv_ptr
        }
    }
}

impl core::ops::DerefMut for PaddedTextView {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe {
            let tv_ptr = &mut self.data as *mut [u8; TV_SIZE] as *mut TextView;
            &mut *tv_ptr
        }
    }
}

#[test]
fn padded_textview() {
    let tv = TextView::default();
    println!("Padded textview: {:?}", tv);

    let mut tv_msg = tv.into_message();
    tv_msg.draw_border = true;

    let original_tv = tv_msg.to_original();
    println!("Original textview: {:?}", original_tv);
}

#[test]
fn simple_ipc() {
    #[derive(flatipc_derive::XousIpc, Debug)]
    #[repr(C)]
    enum SimpleIpc {
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
