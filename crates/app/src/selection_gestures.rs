//! Reshaping and moving a selection on the plot: its edges, its inside and Shift+click.

use super::navigation_ui::{PlotGeometry, Selecting, hertz};
use super::plot_view::{PlotSnapshot, PlotView};
use super::*;
use argand_core::{FrequencyBand, SampleSpan, Selection};
use gpui_kit::{CursorStyle, Point};

/// How near an edge, in logical pixels, a press grabs it.
const EDGE_ZONE: f32 = 4.;

/// One of a selection's two edges along an axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Edge {
    /// The start in time, the low edge in frequency.
    Low,
    High,
}

/// What a press on a selection takes hold of.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Grab {
    /// An edge along time, along frequency, or both at a corner.
    Edges {
        time: Option<Edge>,
        frequency: Option<Edge>,
    },
    /// The whole selection.
    Inside,
}

/// Where a selection lies along one axis, in fractions of what is shown and in pixels per fraction.
#[derive(Debug, Clone, Copy)]
struct Extent {
    low: f64,
    high: f64,
    pixels: f64,
}

impl Extent {
    /// The edge within the zone of `at`, the nearer one when both are.
    fn edge_at(self, at: f64) -> Option<Edge> {
        let distance = |edge: f64| (at - edge).abs() * self.pixels;
        let (low, high) = (distance(self.low), distance(self.high));
        let zone = f64::from(EDGE_ZONE);
        match (low <= zone, high <= zone) {
            (false, false) => None,
            (true, false) => Some(Edge::Low),
            (false, true) => Some(Edge::High),
            (true, true) if low <= high => Some(Edge::Low),
            (true, true) => Some(Edge::High),
        }
    }

    /// Whether `at` lies along the extent, its edge zones included.
    fn spans(self, at: f64) -> bool {
        let zone = f64::from(EDGE_ZONE) / self.pixels;
        (self.low - zone..=self.high + zone).contains(&at)
    }

    fn holds(self, at: f64) -> bool {
        (self.low..=self.high).contains(&at)
    }
}

/// Where on the plot a press lands, for the selection's purposes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Place {
    Spectrum,
    FrequencyRuler,
    Minimap,
}

/// The selection's extents on the spectrogram, the whole axis standing in for one it does not select.
fn extents(
    snapshot: &PlotSnapshot,
    geometry: PlotGeometry,
    minimap: bool,
) -> Option<(Option<Extent>, Option<Extent>)> {
    let selected = snapshot.selected;
    if selected.time.is_none() && selected.band.is_none() {
        return None;
    }
    let time = selected.time.map(|span| {
        let (start, len, pixels) = match minimap {
            true => {
                let length = geometry
                    .orientation
                    .axes(geometry.minimap.size.width, geometry.minimap.size.height)
                    .0;
                (0, snapshot.extents.time.total, f32::from(length))
            }
            false => {
                let view = snapshot.extents.time.view;
                (view.start, view.len, geometry.time_length())
            }
        };
        let at = |sample: u64| (sample as f64 - start as f64) / len.max(1) as f64;
        Extent {
            low: at(span.start()),
            high: at(span.end()),
            pixels: f64::from(pixels),
        }
    });
    let frequency = selected.band.map(|band| {
        let (low, high) = snapshot.extents.hertz;
        let from_top = |hertz: f64| (high - hertz) / (high - low);
        Extent {
            low: from_top(band.high()),
            high: from_top(band.low()),
            pixels: f64::from(geometry.frequency_length()),
        }
    });
    Some((time, frequency))
}

/// What a press at `position` takes hold of, edges before the inside.
fn grab(snapshot: &PlotSnapshot, geometry: PlotGeometry, position: Point<Pixels>) -> Option<Grab> {
    let place = if geometry.controls_at(position) {
        return None;
    } else if geometry.selects_at(position) {
        Place::Spectrum
    } else if geometry.frequency_ruler.contains(&position) {
        Place::FrequencyRuler
    } else if geometry.minimap.contains(&position) {
        Place::Minimap
    } else {
        return None;
    };
    let (time, frequency) = extents(snapshot, geometry, place == Place::Minimap)?;
    let (at_time, at_frequency) = match place {
        Place::Minimap => (geometry.minimap_fraction(position), 0.),
        _ => geometry.fractions(position),
    };
    // An edge is grabbed along the other axis's extent, or anywhere where that axis is not selected or not shown.
    let along_frequency =
        place == Place::Minimap || frequency.is_none_or(|extent| extent.spans(at_frequency));
    let along_time = time.is_none_or(|extent| extent.spans(at_time));
    let time_edge = time
        .filter(|_| place != Place::FrequencyRuler && along_frequency)
        .and_then(|extent| extent.edge_at(at_time));
    let frequency_edge = frequency
        .filter(|_| place != Place::Minimap && (place == Place::FrequencyRuler || along_time))
        .and_then(|extent| extent.edge_at(at_frequency));
    // Frequency runs down the screen, so its low fraction is the band's high edge.
    let frequency_edge = frequency_edge.map(|edge| match edge {
        Edge::Low => Edge::High,
        Edge::High => Edge::Low,
    });
    if time_edge.is_some() || frequency_edge.is_some() {
        return Some(Grab::Edges {
            time: time_edge,
            frequency: frequency_edge,
        });
    }
    let inside = place == Place::Spectrum
        && time.is_none_or(|extent| extent.holds(at_time))
        && frequency.is_none_or(|extent| extent.holds(at_frequency));
    inside.then_some(Grab::Inside)
}

/// The cursor that tells what a press would take hold of.
pub(super) fn grab_cursor(grab: Grab, orientation: crate::orientation::Mode) -> CursorStyle {
    let vertical = orientation.vertical();
    match grab {
        Grab::Inside => CursorStyle::OpenHand,
        Grab::Edges {
            time: Some(time),
            frequency: Some(frequency),
        } => {
            // Time runs right or down, frequency up or right, which decides the corner's diagonal.
            let rising = (time == Edge::High) == (frequency == Edge::High);
            match rising != vertical {
                true => CursorStyle::ResizeUpRightDownLeft,
                false => CursorStyle::ResizeUpLeftDownRight,
            }
        }
        Grab::Edges { time: Some(_), .. } if vertical => CursorStyle::ResizeUpDown,
        Grab::Edges { time: Some(_), .. } => CursorStyle::ResizeLeftRight,
        Grab::Edges { .. } if vertical => CursorStyle::ResizeLeftRight,
        Grab::Edges { .. } => CursorStyle::ResizeUpDown,
    }
}

/// `span` moved by `shift` samples, rounded, and held inside a capture of `total` samples.
fn moved_span(span: SampleSpan, shift: f64, total: u64) -> SampleSpan {
    // The shift alone goes through floating point, so a start near the end of a huge capture stays exact.
    let shift = shift.round() as i128;
    let last = i128::from(total.saturating_sub(span.count()));
    let start = (i128::from(span.start()) + shift).clamp(0, last) as u64;
    SampleSpan::between(start, start + span.count()).unwrap_or(span)
}

/// The edge of `span` that does not move when `edge` does.
fn other_time(span: SampleSpan, edge: Edge) -> u64 {
    match edge {
        Edge::Low => span.end(),
        Edge::High => span.start(),
    }
}

fn other_frequency(band: FrequencyBand, edge: Edge) -> f64 {
    match edge {
        Edge::Low => band.high(),
        Edge::High => band.low(),
    }
}

impl PlotView {
    /// The cursor the selection under a still pointer calls for, none away from it.
    pub(super) fn hover_cursor(&self) -> Option<CursorStyle> {
        let (position, geometry) = self.hover()?;
        let snapshot = self.snapshot.as_ref()?;
        if self.space || self.dragging() {
            return None;
        }
        if self.badge_at(position).is_some() {
            return Some(CursorStyle::Arrow);
        }
        grab(snapshot, geometry, position).map(|grab| grab_cursor(grab, geometry.orientation))
    }

    /// The gesture a left press starts on the selection, if it lands on one or Shift extends it.
    pub(super) fn grab_selection(
        &self,
        event: &gpui_kit::MouseDownEvent,
        geometry: PlotGeometry,
    ) -> Option<Selecting> {
        let snapshot = self.snapshot.as_ref()?;
        if event.modifiers.shift {
            return self.extend_from_nearer_edges(event, geometry, snapshot);
        }
        // Ctrl draws a new rectangle wherever it is pressed.
        if event.modifiers.control {
            return None;
        }
        let grabbed = grab(snapshot, geometry, event.position)?;
        let selected = snapshot.selected;
        let mut selecting = Selecting {
            anchor: None,
            anchor_hz: None,
            origin: event.position,
            moved: false,
            keep_time: selected.time,
            keep_band: selected.band,
            minimap: geometry.minimap.contains(&event.position),
            clears: false,
            moving: None,
            cursor: grab_cursor(grabbed, geometry.orientation),
        };
        match grabbed {
            Grab::Inside => {
                selecting.moving = Some(selected);
                selecting.clears = true;
                selecting.cursor = CursorStyle::ClosedHand;
            }
            Grab::Edges { time, frequency } => {
                if let (Some(edge), Some(span)) = (time, selected.time) {
                    selecting.anchor = Some(other_time(span, edge));
                }
                if let (Some(edge), Some(band)) = (frequency, selected.band) {
                    selecting.anchor_hz = Some(other_frequency(band, edge));
                }
            }
        }
        Some(selecting)
    }

    /// Shift+press moves the nearer edge of each selected axis to the pointer and holds it there.
    fn extend_from_nearer_edges(
        &self,
        event: &gpui_kit::MouseDownEvent,
        geometry: PlotGeometry,
        snapshot: &PlotSnapshot,
    ) -> Option<Selecting> {
        let on_spectrum = geometry.selects_at(event.position);
        let on_ruler = geometry.frequency_ruler.contains(&event.position);
        if !on_spectrum && !on_ruler {
            return None;
        }
        let selected = snapshot.selected;
        let (time, frequency) = geometry.fractions(event.position);
        let sample = snapshot.extents.time.view.boundary(time);
        let hertz = hertz(snapshot.extents.hertz, frequency);
        let anchor = selected.time.filter(|_| on_spectrum).map(|span| {
            let nearer = match sample.abs_diff(span.start()) <= sample.abs_diff(span.end()) {
                true => Edge::Low,
                false => Edge::High,
            };
            other_time(span, nearer)
        });
        let anchor_hz = selected.band.map(|band| {
            let nearer = match (hertz - band.low()).abs() <= (hertz - band.high()).abs() {
                true => Edge::Low,
                false => Edge::High,
            };
            other_frequency(band, nearer)
        });
        if anchor.is_none() && anchor_hz.is_none() {
            return None;
        }
        Some(Selecting {
            anchor,
            anchor_hz,
            origin: event.position,
            moved: true,
            keep_time: selected.time,
            keep_band: selected.band,
            minimap: false,
            clears: false,
            moving: None,
            cursor: CursorStyle::Crosshair,
        })
    }

    /// `from` moved by the pointer's travel from `origin` to `position`, held inside the capture.
    pub(super) fn moved_selection(
        &self,
        from: Selection,
        origin: Point<Pixels>,
        position: Point<Pixels>,
    ) -> Option<Selection> {
        let (geometry, snapshot) = (self.geometry?, self.snapshot.as_ref()?);
        let (time_from, frequency_from) = geometry.fractions(origin);
        let (time_to, frequency_to) = geometry.fractions(position);
        let view = snapshot.extents.time.view;
        let total = snapshot.extents.time.total;
        let time = from
            .time
            .map(|span| moved_span(span, (time_to - time_from) * view.len as f64, total));
        let (low, high) = snapshot.extents.hertz;
        let band = from.band.map(|band| {
            let shift = -(frequency_to - frequency_from) * (high - low);
            let shift = match snapshot.capture_band {
                Some((floor, ceiling)) => shift.clamp(floor - band.low(), ceiling - band.high()),
                None => shift,
            };
            FrequencyBand::between(band.low() + shift, band.high() + shift).unwrap_or(band)
        });
        Some(Selection { time, band })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_nearer_edge_is_grabbed_within_its_zone() {
        let extent = Extent {
            low: 0.25,
            high: 0.75,
            pixels: 400.,
        };
        assert_eq!(extent.edge_at(0.25 + 3. / 400.), Some(Edge::Low));
        assert_eq!(extent.edge_at(0.75 - 3.9 / 400.), Some(Edge::High));
        assert_eq!(extent.edge_at(0.75 - 4.1 / 400.), None);
        assert_eq!(extent.edge_at(0.5), None);
        let narrow = Extent {
            low: 0.5,
            high: 0.505,
            pixels: 400.,
        };
        assert_eq!(narrow.edge_at(0.504), Some(Edge::High));
        assert!(extent.spans(0.25 - 3. / 400.) && !extent.spans(0.2));
    }

    #[test]
    fn a_moved_span_stays_exact_and_inside_a_huge_capture() {
        let span = SampleSpan::between(u64::MAX - (1 << 20), u64::MAX).unwrap();
        assert_eq!(moved_span(span, 0., u64::MAX), span);
        assert_eq!(moved_span(span, 1e6, u64::MAX), span, "held at the end");
        let moved = moved_span(span, -3., u64::MAX);
        assert_eq!(moved.start(), span.start() - 3);
        let early = SampleSpan::between(5, 10).unwrap();
        assert_eq!(moved_span(early, -100., 1000).start(), 0);
    }

    #[test]
    fn corners_and_edges_show_their_resize_cursors_in_both_orientations() {
        use crate::orientation::Mode;
        let corner = |time, frequency| Grab::Edges {
            time: Some(time),
            frequency: Some(frequency),
        };
        assert_eq!(
            grab_cursor(corner(Edge::High, Edge::High), Mode::Horizontal),
            CursorStyle::ResizeUpRightDownLeft
        );
        assert_eq!(
            grab_cursor(corner(Edge::Low, Edge::High), Mode::Horizontal),
            CursorStyle::ResizeUpLeftDownRight
        );
        assert_eq!(
            grab_cursor(corner(Edge::High, Edge::High), Mode::Vertical),
            CursorStyle::ResizeUpLeftDownRight
        );
        let time = Grab::Edges {
            time: Some(Edge::Low),
            frequency: None,
        };
        assert_eq!(
            grab_cursor(time, Mode::Horizontal),
            CursorStyle::ResizeLeftRight
        );
        assert_eq!(grab_cursor(time, Mode::Vertical), CursorStyle::ResizeUpDown);
        assert_eq!(
            grab_cursor(Grab::Inside, Mode::Vertical),
            CursorStyle::OpenHand
        );
    }
}
