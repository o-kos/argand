fn remove_reader_for_test(fail_reader_open: &mut bool, path: &std::path::Path) {
    if std::mem::take(fail_reader_open) {
        std::fs::remove_file(path).expect("remove published scratch before reader preparation");
    }
}
