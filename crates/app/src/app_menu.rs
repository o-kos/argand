//! Toolkit-independent shape of the File submenu.

/// One row of the File submenu, in the order it is drawn.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Row<T> {
    /// Asks the desktop for a file.
    Open,
    /// Writes the edits over the open file.
    Save,
    /// Saves the whole capture as a new file.
    SaveAs,
    /// Saves the time selection as a new file.
    SaveSelectionAs,
    /// A capture from the recent list, with the digit its row promises.
    Recent {
        number: Option<u8>,
        label: String,
        origin: T,
    },
    /// Opens capture properties and conversion options.
    Information,
    Separator,
}

/// How many recent rows promise a digit, 1 to 9 and then 0, as a keyboard row runs.
pub const DIGITS: usize = 10;

/// The digit the shown recent row `index` promises, none past the tenth.
pub fn digit(index: usize) -> Option<u8> {
    match index {
        0..9 => Some(index as u8 + 1),
        9 => Some(0),
        _ => None,
    }
}

/// The rows of the File submenu, in the order they are drawn.
///
/// The recent rows keep the order they arrive in. An empty list leaves one
/// separator between Open and Save, without an empty history group.
pub fn file_items<T>(recent: Vec<(String, T)>) -> Vec<Row<T>> {
    let mut rows = vec![Row::Open, Row::Separator];
    if !recent.is_empty() {
        rows.extend(
            recent
                .into_iter()
                .enumerate()
                .map(|(index, (label, origin))| Row::Recent {
                    number: digit(index),
                    label,
                    origin,
                }),
        );
        rows.push(Row::Separator);
    }
    rows.extend([
        Row::Save,
        Row::SaveAs,
        Row::SaveSelectionAs,
        Row::Separator,
        Row::Information,
    ]);
    rows
}
