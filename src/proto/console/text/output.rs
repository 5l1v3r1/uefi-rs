use {Status, Result};

/// Interface for text-based output devices.
#[repr(C)]
pub struct Output {
    reset: extern "C" fn(this: &Output, extended: bool) -> Status,
    output_string: extern "C" fn(this: &Output, string: *const u16) -> Status,
    test_string: extern "C" fn(this: &Output, string: *const u16) -> Status,
    query_mode: extern "C" fn(this: &Output,
                              mode: i32,
                              columns: &mut usize,
                              rows: &mut usize)
                              -> Status,
    set_mode: extern "C" fn(this: &mut Output, mode: i32) -> Status,
    set_attribute: usize,
    clear_screen: usize,
    set_cursor_position: usize,
    enable_cursor: extern "C" fn(this: &mut Output, visible: bool) -> Status,
    data: &'static OutputData,
}

impl Output {
    /// Resets the text output device hardware.
    pub fn reset(&mut self, extended: bool) -> Result<()> {
        (self.reset)(self, extended).into()
    }

    /// Writes a string to the output device.
    pub fn output_string(&mut self, string: *const u16) -> Result<()> {
        (self.output_string)(self, string).into()
    }

    /// Checks if a string contains only supported characters.
    /// True indicates success.
    ///
    /// UEFI applications are encouraged to try to print a string even if it contains
    /// some unsupported characters.
    pub fn test_string(&mut self, string: *const u16) -> bool {
        match (self.test_string)(self, string) {
            Status::Success => true,
            _ => false,
        }
    }

    /// Returns an iterator of all supported text modes.
    // TODO: fix the ugly lifetime parameter.
    pub fn modes<'a>(&'a mut self) -> impl Iterator<Item = OutputMode> + 'a {
        let max = self.data.max_mode;
        OutputModeIter {
            output: self,
            current: 0,
            max,
        }
    }

    /// Returns the width (column count) and height (row count) of this mode.
    fn query_mode(&self, index: i32) -> Result<(usize, usize)> {
        let (mut columns, mut rows) = (0, 0);
        (self.query_mode)(self, index, &mut columns, &mut rows)?;
        Ok((columns, rows))
    }

    /// Sets a mode as current.
    pub fn set_mode(&mut self, mode: OutputMode) -> Result<()> {
        (self.set_mode)(self, mode.index).into()
    }

    /// Returns the the current text mode.
    pub fn current_mode(&self) -> Result<OutputMode> {
        let index = self.data.mode;
        let dims = self.query_mode(index)?;
        Ok(OutputMode { index, dims })
    }

    /// Enables or disables the cursor.
    pub fn enable_cursor(&mut self, visible: bool) -> Result<()> {
        (self.enable_cursor)(self, visible).into()
    }
}

/// The text mode (resolution) of the output device.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct OutputMode {
    index: i32,
    dims: (usize, usize),
}

impl OutputMode {
    /// Returns the index of this mode.
    #[inline]
    pub fn index(&self) -> i32 {
        self.index
    }

    /// Returns the width in columns.
    #[inline]
    pub fn columns(&self) -> usize {
        self.dims.0
    }

    /// Returns the height in rows.
    #[inline]
    pub fn rows(&self) -> usize {
        self.dims.1
    }
}

/// An iterator of the text modes (possibly) supported by a device.
struct OutputModeIter<'a> {
    output: &'a mut Output,
    current: i32,
    max: i32,
}

impl<'a> Iterator for OutputModeIter<'a> {
    type Item = OutputMode;

    fn next(&mut self) -> Option<Self::Item> {
        let index = self.current;
        if index < self.max {
            self.current += 1;

            if let Ok(dims) = self.output.query_mode(index) {
                Some(OutputMode { index, dims })
            } else {
                self.next()
            }
        } else {
            None
        }
    }
}

/// Additional data of the output device.
#[derive(Debug)]
#[repr(C)]
struct OutputData {
    /// The number of modes supported by the device.
    max_mode: i32,
    /// The current output mode.
    mode: i32,
    /// The current character output attribute.
    attribute: i32,
    /// The cursor’s column.
    cursor_column: i32,
    /// The cursor’s row.
    cursor_row: i32,
    /// Whether the cursor is currently visible or not.
    cursor_visible: bool,
}

impl_proto! {
    protocol Output {
        GUID = 0x387477c2, 0x69c7, 0x11d2, [0x8e, 0x39, 0x00, 0xa0, 0xc9, 0x69, 0x72, 0x3b];
    }
}
