#[cfg(test)]
fn rebin(store: &ColumnStore, time: &Overlaps, frequency: &Overlaps) -> Vec<f32> {
    let filled: Vec<_> = (0..store.width).collect();
    time.0
        .iter()
        .flat_map(|spans| {
            frequency
                .0
                .iter()
                .map(|rows| reduced_value(store, &filled, spans, rows))
        })
        .collect()
}
