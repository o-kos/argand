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
fn the_first_ten_recent_rows_promise_one_to_nine_then_zero() {
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
    assert_eq!(numbers[9..], [Some(0), None]);
    assert_eq!(digit(DIGITS), None);
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
