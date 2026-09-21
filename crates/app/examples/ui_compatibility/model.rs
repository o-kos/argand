#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Orientation {
    #[default]
    Horizontal,
    Vertical,
}

impl Orientation {
    pub fn toggled(self) -> Self {
        match self {
            Self::Horizontal => Self::Vertical,
            Self::Vertical => Self::Horizontal,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Horizontal => "Horizontal",
            Self::Vertical => "Vertical",
        }
    }

    const fn index(self) -> usize {
        match self {
            Self::Horizontal => 0,
            Self::Vertical => 1,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlotPoint {
    pub time: f32,
    pub frequency: f32,
}

impl PlotPoint {
    pub fn new(time: f32, frequency: f32) -> Self {
        Self {
            time: time.clamp(0.0, 1.0),
            frequency: frequency.clamp(0.0, 1.0),
        }
    }

    pub fn readout(self) -> String {
        let seconds = self.time * 10.0;
        let megahertz = 99.5 + self.frequency;
        let level = -120.0 + (1.0 - self.frequency) * 100.0;
        format!("{seconds:.3} s  |  {megahertz:.6} MHz  |  {level:.1} dB")
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ZoomDirection {
    #[default]
    In,
    Out,
}

pub const PROBE_NUMBER_MIN: i64 = 64;
pub const PROBE_NUMBER_MAX: i64 = 4_096;
pub const PROBE_NUMBER_STEP: i64 = 64;
pub const PROBE_NUMBER_DEFAULT: i64 = 2_048;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumberStep {
    Decrement,
    Increment,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumberValidation {
    Valid(i64),
    NotInteger,
    OutOfRange,
}

impl NumberValidation {
    pub fn message(self) -> String {
        match self {
            Self::Valid(value) => format!(
                "Valid fixture value: {value} (range {PROBE_NUMBER_MIN}..={PROBE_NUMBER_MAX})"
            ),
            Self::NotInteger => {
                format!("Invalid: enter an integer from {PROBE_NUMBER_MIN} to {PROBE_NUMBER_MAX}")
            }
            Self::OutOfRange => {
                format!("Invalid: value must be from {PROBE_NUMBER_MIN} to {PROBE_NUMBER_MAX}")
            }
        }
    }

    pub fn is_valid(self) -> bool {
        matches!(self, Self::Valid(_))
    }
}

impl Default for NumberValidation {
    fn default() -> Self {
        Self::Valid(PROBE_NUMBER_DEFAULT)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counters {
    pub pointer_moves: u64,
    pub pointer_presses: u64,
    pub clicks: u64,
    pub wheel_events: u64,
    pub zoom_in: u64,
    pub zoom_out: u64,
    pub menu_actions: u64,
    pub button_actions: u64,
    pub plot_key_actions: u64,
    pub number_step_events: u64,
    pub resize_callbacks: [u64; 2],
}

#[derive(Debug, Default)]
pub struct ProbeModel {
    pub orientation: Orientation,
    pub pointer: Option<PlotPoint>,
    pub selection: Option<(PlotPoint, PlotPoint)>,
    pub counters: Counters,
    pub zoom_steps: i8,
    pub last_menu_action: Option<&'static str>,
    pub plot_hovered: bool,
    pub passive_hint_hovered: bool,
    pub number_validation: NumberValidation,
}

impl ProbeModel {
    pub fn move_pointer(&mut self, point: PlotPoint, dragging: bool) {
        self.counters.pointer_moves += 1;
        self.pointer = Some(point);
        if dragging && let Some((start, _)) = self.selection {
            self.selection = Some((start, point));
        }
    }

    pub fn press_pointer(&mut self, point: PlotPoint) {
        self.counters.pointer_presses += 1;
        self.pointer = Some(point);
        self.selection = Some((point, point));
    }

    pub fn click_pointer(&mut self, point: PlotPoint) {
        self.counters.clicks += 1;
        self.pointer = Some(point);
    }

    pub fn wheel(&mut self) {
        self.counters.wheel_events += 1;
    }

    pub fn zoom(&mut self, direction: ZoomDirection) {
        match direction {
            ZoomDirection::In => {
                self.counters.zoom_in += 1;
                self.zoom_steps = self.zoom_steps.saturating_add(1).min(8);
            }
            ZoomDirection::Out => {
                self.counters.zoom_out += 1;
                self.zoom_steps = self.zoom_steps.saturating_sub(1).max(-8);
            }
        }
    }

    pub fn toggle_orientation(&mut self) {
        self.orientation = self.orientation.toggled();
    }

    pub fn record_menu_action(&mut self, action: &'static str) {
        self.counters.menu_actions += 1;
        self.last_menu_action = Some(action);
    }

    pub fn record_button_action(&mut self) {
        self.counters.button_actions += 1;
    }

    pub fn record_plot_key_action(&mut self) {
        self.counters.plot_key_actions += 1;
    }

    pub fn set_passive_hint_hovered(&mut self, hovered: bool) {
        self.passive_hint_hovered = hovered;
    }

    pub fn set_plot_hovered(&mut self, hovered: bool) {
        self.plot_hovered = hovered;
    }

    pub fn validate_number(&mut self, text: &str) {
        self.number_validation = validate_number(text);
    }

    pub fn step_number(&mut self, text: &str, direction: NumberStep) -> Option<String> {
        self.counters.number_step_events += 1;
        self.number_validation = validate_number(text);
        let NumberValidation::Valid(value) = self.number_validation else {
            return None;
        };
        let delta = match direction {
            NumberStep::Decrement => -PROBE_NUMBER_STEP,
            NumberStep::Increment => PROBE_NUMBER_STEP,
        };
        let value = (value + delta).clamp(PROBE_NUMBER_MIN, PROBE_NUMBER_MAX);
        self.number_validation = NumberValidation::Valid(value);
        Some(value.to_string())
    }

    pub fn record_resize_callback(&mut self, orientation: Orientation) {
        self.counters.resize_callbacks[orientation.index()] += 1;
    }

    pub fn resize_callbacks(&self, orientation: Orientation) -> u64 {
        self.counters.resize_callbacks[orientation.index()]
    }

    pub fn reset_counters(&mut self) {
        self.counters = Counters::default();
    }
}

fn validate_number(text: &str) -> NumberValidation {
    let Ok(value) = text.trim().parse::<i64>() else {
        return NumberValidation::NotInteger;
    };
    if !(PROBE_NUMBER_MIN..=PROBE_NUMBER_MAX).contains(&value) {
        return NumberValidation::OutOfRange;
    }
    NumberValidation::Valid(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pointer_drag_updates_selection_and_named_counters() {
        let mut model = ProbeModel::default();
        let start = PlotPoint::new(0.2, 0.8);
        let end = PlotPoint::new(0.7, 0.3);

        model.press_pointer(start);
        model.move_pointer(end, true);
        model.click_pointer(end);
        model.wheel();

        assert_eq!(model.pointer, Some(end));
        assert_eq!(model.selection, Some((start, end)));
        assert_eq!(model.counters.pointer_presses, 1);
        assert_eq!(model.counters.pointer_moves, 1);
        assert_eq!(model.counters.clicks, 1);
        assert_eq!(model.counters.wheel_events, 1);
    }

    #[test]
    fn plot_points_are_bounded_and_have_physical_readouts() {
        let point = PlotPoint::new(-0.5, 2.0);

        assert_eq!(point, PlotPoint::new(0.0, 1.0));
        assert_eq!(point.readout(), "0.000 s  |  100.500000 MHz  |  -120.0 dB");
    }

    #[test]
    fn orientation_keeps_independent_resize_callback_counts() {
        let mut model = ProbeModel::default();
        model.record_resize_callback(Orientation::Horizontal);
        model.toggle_orientation();
        model.record_resize_callback(model.orientation);
        model.record_resize_callback(model.orientation);

        assert_eq!(model.orientation, Orientation::Vertical);
        assert_eq!(model.resize_callbacks(Orientation::Horizontal), 1);
        assert_eq!(model.resize_callbacks(Orientation::Vertical), 2);
    }

    #[test]
    fn counter_reset_preserves_probe_state() {
        let mut model = ProbeModel::default();
        model.press_pointer(PlotPoint::new(0.25, 0.75));
        model.zoom(ZoomDirection::In);
        model.record_menu_action("Oceanic");
        model.toggle_orientation();

        model.reset_counters();

        assert_eq!(model.counters, Counters::default());
        assert_eq!(model.orientation, Orientation::Vertical);
        assert_eq!(model.zoom_steps, 1);
        assert_eq!(model.last_menu_action, Some("Oceanic"));
        assert!(model.selection.is_some());
    }

    #[test]
    fn zoom_is_bounded_without_hiding_attempt_counts() {
        let mut model = ProbeModel::default();
        for _ in 0..20 {
            model.zoom(ZoomDirection::In);
        }

        assert_eq!(model.zoom_steps, 8);
        assert_eq!(model.counters.zoom_in, 20);
    }

    #[test]
    fn bounded_number_validation_keeps_invalid_text_observable() {
        let mut model = ProbeModel::default();

        model.validate_number("not a number");
        assert_eq!(model.number_validation, NumberValidation::NotInteger);

        model.validate_number("63");
        assert_eq!(model.number_validation, NumberValidation::OutOfRange);

        model.validate_number(" 2048 ");
        assert_eq!(model.number_validation, NumberValidation::Valid(2_048));
    }

    #[test]
    fn number_steps_are_bounded_and_count_rejected_events() {
        let mut model = ProbeModel::default();

        assert_eq!(
            model.step_number("2048", NumberStep::Increment),
            Some("2112".into())
        );
        assert_eq!(
            model.step_number("4096", NumberStep::Increment),
            Some("4096".into())
        );
        assert_eq!(model.step_number("invalid", NumberStep::Decrement), None);
        assert_eq!(model.number_validation, NumberValidation::NotInteger);
        assert_eq!(model.counters.number_step_events, 3);
    }

    #[test]
    fn reset_preserves_validation_and_hover_observations() {
        let mut model = ProbeModel::default();
        model.validate_number("5000");
        model.set_plot_hovered(true);
        model.set_passive_hint_hovered(true);
        model.record_plot_key_action();

        model.reset_counters();

        assert_eq!(model.counters, Counters::default());
        assert_eq!(model.number_validation, NumberValidation::OutOfRange);
        assert!(model.plot_hovered);
        assert!(model.passive_hint_hovered);
    }
}
