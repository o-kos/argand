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
    /// Opens the analysis settings.
    Settings,
    Separator,
}

/// How many recent rows promise a digit, as the start page's own list does.
const DIGITS: usize = 9;

/// The rows of the File submenu, in the order they are drawn.
///
/// The recent rows keep the order they arrive in. An empty list leaves one
/// separator, between the two commands and nothing else.
pub fn file_items<T>(recent: Vec<(String, T)>) -> Vec<Row<T>> {
    let mut rows = vec![
        Row::Open,
        Row::Save,
        Row::SaveAs,
        Row::SaveSelectionAs,
        Row::Separator,
    ];
    if !recent.is_empty() {
        rows.extend(
            recent
                .into_iter()
                .enumerate()
                .map(|(index, (label, origin))| Row::Recent {
                    number: (index < DIGITS).then_some((index + 1) as u8),
                    label,
                    origin,
                }),
        );
        rows.push(Row::Separator);
    }
    rows.push(Row::Settings);
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What each row is, without the payload that goes with a capture.
    fn shape(rows: &[Row<u8>]) -> Vec<String> {
        rows.iter()
            .map(|row| match row {
                Row::Open => "open".to_owned(),
                Row::Save => "save over".to_owned(),
                Row::SaveAs => "save".to_owned(),
                Row::SaveSelectionAs => "save selection".to_owned(),
                Row::Settings => "settings".to_owned(),
                Row::Separator => "separator".to_owned(),
                Row::Recent { number, label, .. } => format!("{number:?}:{label}"),
            })
            .collect()
    }

    fn recent(count: usize) -> Vec<(String, u8)> {
        (0..count)
            .map(|index| (format!("capture{index}"), index as u8))
            .collect()
    }

    #[test]
    fn an_empty_recent_list_leaves_one_separator() {
        assert_eq!(
            shape(&file_items::<u8>(vec![])),
            [
                "open",
                "save over",
                "save",
                "save selection",
                "separator",
                "settings"
            ]
        );
    }

    #[test]
    fn the_recent_rows_keep_their_order_between_two_separators() {
        assert_eq!(
            shape(&file_items(recent(2))),
            [
                "open",
                "save over",
                "save",
                "save selection",
                "separator",
                "Some(1):capture0",
                "Some(2):capture1",
                "separator",
                "settings"
            ]
        );
    }

    #[test]
    fn only_the_first_nine_recent_rows_promise_a_digit() {
        let numbers: Vec<Option<u8>> = file_items(recent(11))
            .iter()
            .filter_map(|row| match row {
                Row::Recent { number, .. } => Some(*number),
                _ => None,
            })
            .collect();
        assert_eq!(
            numbers[..9],
            [
                Some(1),
                Some(2),
                Some(3),
                Some(4),
                Some(5),
                Some(6),
                Some(7),
                Some(8),
                Some(9)
            ]
        );
        assert_eq!(numbers[9..], [None, None]);
    }

    #[test]
    fn a_recent_row_carries_the_capture_it_names() {
        let rows = file_items(vec![("capture0".to_owned(), 7u8)]);
        assert_eq!(
            rows[5],
            Row::Recent {
                number: Some(1),
                label: "capture0".to_owned(),
                origin: 7,
            }
        );
    }
}
