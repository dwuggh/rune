#[derive(Debug)]
pub struct Window {
    wid: u64,
    frame_id: u64,

    /// use buffer name instead of lisp objects, we can get
    /// buffer content through `buffer::BUFFERS`
    buffer_name: String,

    /// A marker pointing to where in the text to start displaying.
    start: u64,

    /// A marker pointing to where in the text point is in this window,
    /// used only when the window is not selected.
    /// This exists so that when multiple windows show one buffer
    /// each one can have its own value of point.
    point: u64,

    /// pixel left
    left: u64,
    /// pixel top
    top: u64,

    /// Line number and position of a line somewhere above the top of the
    /// screen.  If this field is zero, it means we don't have a base line.
    ///
    /// used in line-number-mode, ignore for now
    base_line_number: u64,
    base_line_pos: u64,
}
