use super::*;
#[test]
fn os_locale_identifiers_keep_numeric_region_and_numbering() {
    assert_eq!(normalize("ru_RU.UTF-8"), "ru-RU");
    assert_eq!(normalize("C.UTF-8"), "C");
    assert_eq!(normalize("de_DE@calendar=gregorian"), "de-DE");
    assert_eq!(
        normalize("ar_EG@calendar=gregorian;numbers=latn"),
        "ar-EG-u-nu-latn"
    );
}
