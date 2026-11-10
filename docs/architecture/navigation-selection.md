# Navigation and selection

Physical coordinates, plot navigation and selection presentation. Read this before changing gestures, axis layout or typed selection.

## Time navigation (#30)

`navigation/mod.rs` owns bounded sample ranges and held-grid level lookup without GPUI.
The shell starts every opened file at its full sample range, clamps its span to
one FFT or the shorter capture, and sends range changes through the existing
cancellable mailbox. Size and style changes retain their previous cache semantics.
`shell/plot/navigation_ui.rs` provides Ctrl+wheel zoom about the pointer, middle-button and Space+left-drag pan on the spectrum, left-drag pan on the time ruler, identical Ctrl+wheel zoom and wheel horizontal pan over plots and the time ruler, keyboard
commands and the View menu. The crosshair is restricted to the spectrogram. `shell/plot/plot_ui.rs` immediately maps held spectral pictures into
the requested time interval and clips the spectrogram to its plot. Cursor levels
come from the displayed grid's own extents; uncovered time has no reported level.
The completed full-capture minimap updates the file hint's extrema independently
of spectral navigation; partial minimap previews never supply complete extrema.
At zoom ratios above 1024, the few visible source columns are cached as one-pixel-wide textures and drawn
with clipped bounds to avoid losing the viewport in large GPU f32 image coordinates.
`AxisKind::PreciseTime` extends the shared clock ladder with fractional seconds
for the GUI; CLI whole-second clock formatting remains unchanged.
Recent entries contain no time view; deserialization ignores legacy saved ranges. Each file opening starts with the full capture, even within the same run. Navigation gestures perform no filesystem writes.
The minimum span also reserves two floating-point ULPs per display column at
extreme sample counts (at least ten columns for keyboard pans). Width changes
reapply this floor before requesting analysis. Escape in the settings hint restores automatic view expansion; explicit navigation while it is open becomes the new view to restore.
Cursor time precision follows the visible time per pixel, up to nanoseconds.

Every analysis after a file's first one replaces a picture already shown, whether the
range moved or a transform setting changed (#108), and uses
`ProgressiveOptions::final_only` (`Requested::replacement` in `analysis/mod.rs`): sequential
sample reading, no sparse previews or intermediate pictures, and cancellation between
bounded batches. Only the first analysis of an opened file previews and refines, because
it has no picture to keep; an edit keeps its picture moved with the samples (#193). A settings change used to restart the preview sequence, which
replaced a complete picture with a coarse one and changed its brightness several times
in the peak-relative range mode. `analysis/progress.rs` forwards source hints and sends bounded, nonblocking
progress-only notifications from sequential reads every 50 ms. At most 128 frames fitting one configured memory/work-bounded batch use a
single FFT accumulator; larger workloads retain parallel frame groups. Navigation
textures use one logical column for a single FFT frame; multi-frame pictures retain
pixel-width time cells so toolkit filtering cannot blend different FFT frames.
The overview computes and shades identical adjacent time-cell overlaps once and
copies their pixels. Waveform columns still match the plot width. Spectrogram
textures replicate their edge texels and clip the padding at paint time to prevent
linear atlas sampling from bleeding neighbouring allocations into stretched images.
`shell/backdrop.rs` keeps one additional wider picture (CPU grid/image, GPU
texture) to fill only time uncovered by the foreground. It is scoped
to the displayed transform and file. It keeps the widest extent, upgrades a matching
preview on completion, and reports cursor levels only from completed backdrops.
A held preview can seed it when navigation interrupts initial analysis; it remains
an approximate visual placeholder. Style changes shade the retained grid in a
background task and coalesce the pending foreground delivery; both pictures change
style together after delivery-generation validation. Retention defers wider-picture
replacement and completion upgrades while that delivery is parked for shading, so
releasing a backdrop cannot discard a valid foreground result. Equivalent rounded-hop settings
retain it. Both
foreground and backdrop use bounded source-column textures above 1024x stretch.
This spectral navigation placeholder is separate from the full-capture minimap.

`axes::CursorGuides` paints transient Alt-held cursor-to-ruler lines and coordinate
badges from the requested physical extents, with precision based on device pixels. Badges use axis font metrics and remain
within the spectrum panel; they do not change layout or submit analysis. Modifier
and window-activation events repaint the shell, with current window modifier state
read during painting. Guides are absent outside the spectrum, during panning, with
an open menu or while the window is inactive. Frequency navigation is described in [Frequency navigation](navigation-selection.md#frequency-navigation-80).

`axis::TickScheme` retains the selected division spacing and clock-format span while
panning. Zoom, file opening and width changes release that scheme for fresh layout.
If wider labels collide after a pan, the grid remains stable and colliding label text
is omitted. Left/Right pan by one measured division; Ctrl+Left/Right by five.
`navigation::TickPan` accumulates divisions before sample rounding, preventing drift
for nonintegral samples per division; hitting either capture edge resets its origin.
Shift+wheel pans frequency and does not navigate in time. Unmodified wheel pans horizontally; Ctrl+wheel zooms on both
plots and the time ruler.

All time zoom keys use Control: Ctrl+Plus/Equals, Ctrl+Minus and Ctrl+0 (fit).
Alt guides use a three-logical-pixel stroke in the theme's paper colour with a
one-pixel core in its ink colour (#148). That is the same sequence as a badge (ink
fill inside a paper ring), black–white–black in the dark theme. The guides are
clipped to the plot, and the coordinate badges are slightly rounded. Guide colours
follow the interface theme, independent of the spectrogram palette. Badges (#141) wrap the digits' ink with
3 logical pixels on every side. A one-pixel ring in the paper colour (#148) surrounds
each badge so its edge stays visible over picture areas of its own shade. Rings are
painted before the guide lines, which therefore meet the badge fill without a gap. The bottom badge is centred on the bottom
ruler's text row, so its text shares the labels' row and clears the ruler line.
The right badge is centred on the guide line and stands as far from the panel's
right edge as the bottom badge stands from the bottom ruler's edge (#148). Clamping moves a badge only where the
panel has no room; `cursor_guides::badge_rects` holds the geometry.

## Time ruler modes (#81)

`navigation/time_ruler.rs` owns the toolkit-neutral clock/seconds/samples presentation.
`View > Time scale format` and the time-ruler context menu share the mode items; session version 7 remembers it, while older
sessions default to clock and file openings still reset the navigation range.
`AxisKind::Seconds` uses decimal second steps with matching fractional labels;
`AxisKind::Samples` uses integer decimal steps and exact integer-multiple labels
prefixed with `#`. The ruler receives native sample bounds so a complex sample
counts one I/Q pair. Held schemes use the selected ruler's units; keyboard pan
converts its step to samples once. Changing mode clears the held scheme, preserves
the view and performs no analysis request. The Alt time badge uses the same mode;
its sample index is computed relative to the integer view start and clamped to an
existing sample. Alt badge widths are measured from the current extent endpoints
with the same widest-digit font policy and pixel precision as readouts; pointer
motion only changes centered text and position. The shell retains a lazy measurement
cache keyed by extents, plot geometry, display scale and font; it is accessed only
while Alt guides are visible. Numeric locale is immutable after startup.
Guide lines overlap the rounded
badge backgrounds, including across the gap below the spectral image.
Status-bar coordinates share the Alt badge formatter and follow both ruler units. CLI defaults and
CLI frequency axes are unchanged.

`numbers/mod.rs` formats GUI numbers with ICU/CLDR using the regional numeric locale
read by `numbers/system.rs` at startup. Top-level `Config::number_format` defaults
to `system` and accepts an explicit BCP 47 tag or C/POSIX. It has no visual editor
or session override. Configuration loads before numeric initialization, which
precedes session restoration and window creation; invalid locale strings reset
only this field to `system`. Numeric settings use the same formatter
and validate localized grouping when parsing. `LabelMeasure::localize` runs before
tick measurement; CLI implementations keep the default identity hook. GUI HMS
labels use colons between clock fields, and fractions use the locale decimal mark.
Time units are painted once at the right; space for all three captions is reserved
independently of the mode. In horizontal orientation switching the mode therefore
cannot resize the spectral image. In vertical orientation the right gutter refits
the new mode's time labels (#148).
Unit hints use measured caption rectangles from the axis frame. The time hint
belongs to the ruler context-menu element so right-click handling is preserved;
the frequency hint occupies only its visible caption beside the minimap, not the frequency ticks. Hints observe the shell and report the current view span per physical device pixel, in seconds/samples and an independently selected frequency unit (Hz through GHz)
based on resolution magnitude, without redundant decimal zeros. Hand cursors require an available pan range; the frequency ruler supports frequency panning and zooming.
Machine-readable persistence and CLI number formatting do not use this module.

The time-ruler context menu retains its popup entity after closing (a toolkit
reference cycle, #144). `PlotView` tracks its DismissEvent explicitly, clears only the matching menu, and restores
the pointer from the owning window through the ordinary plot filter. This keeps
Alt guides and cursor feedback available without a mouse move after dismissal.

## Frequency navigation (#80)

`navigation/frequency.rs` owns a normalized, bounded frequency viewport independent of the time
range. Each file opening resets it. Its minimum span is one retained frequency
cell, with at most 2048 cells across the capture band and a physical-coordinate
precision floor for extreme centre frequencies. The document mailbox treats
frequency bounds as a display revision, never a new analysis generation.
`Overview::render_band` rebins overlapping native-bin/cache-cell intervals in the
value domain on the worker; full-band rendering retains its existing reduction
policy. Held textures, the wider backdrop and numeric level lookup use both time
and frequency extents. Frequency panning holds the tick scheme until zoom or a
height change. Shift+wheel pans frequency; Ctrl+Shift+wheel zooms it. The frequency
ruler supports drag/wheel pan and Ctrl+wheel zoom. Frequency keyboard zoom uses
Ctrl+Shift+plus/minus and Ctrl+Shift+0; Up/Down and Ctrl+Up/Down pan one/five divisions. No navigation
state is persisted, and the full-capture minimap is independent of both viewports.

## Time selection (#178)

`argand_core::selection` holds `Selection { time, band }`: an optional
`SampleSpan` in whole samples (a complex sample counts once, so I and Q stay
together) and an optional `FrequencyBand` in physical hertz (#202).
The kinds and their later operations are phase 5 of `IMPLEMENTATION_PLAN.md`.
`PlotView::press` decides a gesture once per press: unless Space is held, a left
press selects time on the spectrum, a rectangle with Ctrl there and a band on the
frequency ruler, and every other press on the plot (middle button, Space+left,
left on the time ruler or minimap) pans as before. A pan remembers its
button (`pan_button`), because GPUI's `MouseMoveEvent::dragging` means the left
button only. A selection's boundaries come from `View::boundary` on the time
fraction, so it follows the time axis in both orientations and stops at the
visible edge beyond the plot. A press that moves less than 3 logical pixels is a
click and clears the selection; Escape on the focused plot clears it too
(`ClearSelection`). Space is tracked by key down and up on the plot surface and
forgotten on blur and in `end_gestures`; while it is held the spectrum shows an
open hand, and a selection in progress keeps the crosshair. Shell keeps `selection`, clears it when a
file opens and never persists it; navigation and orientation changes keep it.
The snapshot carries it as view fractions (`View::fractions_of`); the spectrum
dims the picture outside it under the grid, black at 0.55 (`waveform::outside`, #219, the
owner's variant C), which shows it on a bright picture, and tints it with `blue_light` at 0.3,
which shows it on a dark one; the minimap only tints it (`waveform::time_band`, at least one
device pixel wide).

## Selection on the plot (#183)

A selection carries its readouts as badges over the plot (`axes::SelectionBadges`,
`axes/selection_badges.rs`), in the Alt badge look, one `AxisBadges` per selected axis, dimensioned
as on a drawing: each edge's badge just beyond the selection on one side, running from its
edge towards the other so the corners stay free for the other axis. Where the edges are too
close for both, a rectangle shows only that axis's extent, while a selection of one axis
(`Extent::alone`), whose badges meet no others, turns them outward past its edges. Edge badges
that have no room beyond the selection and, pressed inside, would cover the other side's
dimension line stand just past that line instead, at their edges, and give way to the extent
badge where they would meet it. Badges are settled one after another, time's first
(`selection_badges::settle`): an edge badge that would cover one already placed is dropped,
since only its place ties it to its edge, and an extent badge moves along its axis to the
nearest free place (`clear_of`), its dimension line following it. The
extent over the middle of the visible selection just beyond it on the other
(`selection_badges::Layout`), pressed back inside the plot where there is no room beyond it.
Time puts its start and end below the selection and its length above it, or left and right of
it in vertical orientation; a band or a rectangle puts its high and low edges left of it and its
width right of it, or above and below it with frequency running right. A selection that spans
the other axis therefore has its badges at the plot's edges. `time_ruler::Mode::counters` gives the time values
in the ruler's mode with the decimals one pixel of the visible time tells apart, as the Alt
badges do, never more than one sample needs, the clock with the fields the capture's own
length needs; `Scale::band_texts` gives the band's as numbers in the frequency ruler's unit, which
only the ruler names. The extent badge sits on a dimension line (`selection_badges::Dimension`),
a paper stroke around an ink core like the Alt guides, running from edge to edge of the
visible selection with an arrowhead at each edge in view. Where the badge and its arrowheads do
not fit between the edges, the arrowheads stand outside them pointing in, the line runs on past
them, and the badge stands beside the selection away from the other axis's extent
(`Layout::narrow_before`): time's before its start, a band's below it, or before it with
frequency running right, and on the other side without room there. A hidden edge has no badge;
badges stay inside the plot, painted over the grid and under the zoom pairs and the Alt guides,
and take only a left press (see [Typing a selection (#219)](navigation-selection.md#typing-a-selection-219)). Each edge's badge stands a little
farther from the selection (`EDGE_GAP`) and points at its edge with a tail drawn as one shape with
it, inside one paper ring: one side of the tail runs along the edge from the badge's corner there,
squared, to the selection, its base runs from the edge into the badge, and the ring's tip ends on
the selection, so two edges meeting at a corner keep their tails apart (`selection_badges::pointer`). A badge pressed against the
selection stands far enough from it that the badge input opened over it keeps one logical pixel
clear (`Extent::clear`, from `badge_entry::entry_size`). The status bar shows no selection.

`shell/plot/selection_gestures.rs` reshapes and moves a selection; each gesture is a `Selecting`, so
every place that ends a gesture ends these too. A left press without Ctrl within 4 logical
pixels of an edge grabs it (`Grab::Edges`): a time edge on the spectrum along the band's
extent and on the minimap, a band edge on the spectrum along the time extent and on the
frequency ruler, both at a corner. The other edge is the anchor and the axis not grabbed is
kept (`Selecting::keep_time`, `keep_band`), so dragging past the other edge swaps them and an
edge laid on its partner leaves the selection as it was; a click on an edge changes nothing.
Inside the selection on the spectrum (`Grab::Inside`) a drag moves it whole
(`PlotView::moved_selection`), its length and width kept, held to the capture's time and band;
a click there clears it. Shift+press moves the nearer edge of each selected axis to the
pointer and keeps dragging it. The pointer shows the resize cursor across the grabbed axis,
a diagonal at a corner, an open hand inside and a closed hand while moving
(`selection_gestures::grab_cursor`, `PlotView::hover_cursor`). Space and the middle button
still pan everywhere.

## Typing a selection (#219)

`navigation/selection_entry.rs` (toolkit-neutral) writes and reads a selection's six values
(`Field::{TimeStart, TimeEnd, TimeLength, BandLow, BandHigh, BandWidth}`) against a `Scale`
the shell gives (`Shell::entry_scale`), which the badges' own texts come from too, so the badges,
the badge input and the dialog show one value alike: time in the ruler's mode to one pixel of the
visible time, frequency in the ruler's unit to one row, numbers through the numeric locale. A
text left unchanged is never read again, so its rounding moves no edge. A clock
takes one to three colon fields, seconds an optional `s`, frequency an optional unit name, and
a leading `#` is a sample index in every mode, the end naming the last sample; where the mode
writes indexes (`Mode::writes_samples`) a plain number is one. Times round to whole samples,
values are held to the capture, edges swap like a drag past each other, a length or width moves
the end or high edge, and an equal edge or a text that does not parse gives nothing.

The badge layout is `SelectionBadges::place`, run in the canvas's prepaint; each badge carries
its `Field` and `BadgeLayout` hands the rectangles to `PlotView::place_badges`. A left press on a
badge (`PlotView::badge_at`) starts no gesture and the pointer is an arrow there; a double click
opens a `BadgeEntry` (`shell/badge_entry.rs`), an extra-small stock `Input` with `appearance(false)` in
the badge's type and padding, keeping its own height centred on the badge in a frame as wide as
the badge with room for the caret, a sibling of the plot
surface so no `Plot` binding matches while it holds the keyboard, and the badge under it is not
painted. Enter applies it as `PlotIntent::Select` or marks the text it refuses in the danger
colour, Escape (the `BadgeEntry` context, since the input passes it on) and blur close it
unchanged, unchanged text applies nothing, and a selection changed elsewhere closes it.

The selection panel (`shell/plot/selection_panel.rs`), which replaced a modal dialog at the owner's
request, is a `PinnedHint` hanging under three title-bar segments right of Grid, Time, Band and
Both (`app_menu_ui::selection_segments`, toolbar-styled `Button`s in a `ButtonGroup` with the
Lucide `move-horizontal`, `move-vertical` and `square-dashed` artwork, the arrows running along
the axis they select as the plot turns). The segment of the kind selected is lit; a segment of
another kind selects it, taking a missing axis from the visible range, and opens the panel, and
the lit one toggles the panel (`Shell::choose_selection_kind`). Each segment takes its own click,
since a `ButtonGroup`'s handler hears the mouse only. Ctrl+E (Cmd+E, `EditSelection`, in the
`Plot` context and in the panel's own, where it closes the panel as Enter does) and Edit → Selection panel toggle it too; with nothing selected they open it on
time and band, both from the visible range. The lit segment is `selected`, as the orientation
segment in force is, so both groups light alike. `hints::pinned` takes the trigger and
the anchor, so the panel hangs below its trigger where the analysis hint stands above its own.

The panel draws the selection as the badges show it, with the six `Input`s placed around it, the
start and end under it, the length on a dimension line above, the low and high edges at its left
and the width on a dimension line at its right (`paint_dimension`), each line naming its unit
past its end, in the hint's own look and type. With the plot turned (`Drawing::vertical`) it sides them as the turned plot's
badges: the start and end left of it, the length on a dimension line at its right with the time
unit above, the low and high edges above it and the width on a dimension line below. Each field is the standard `Input` with `appearance(false)` in a frame
the drawing paints (`axes::callout`), so the four edge fields point at their edges with the
plot badges' tails, the frame in the input's own colours, the ring colour while it holds the
keyboard and the danger colour while it is refused. An axis not selected shows the visible range
muted, and a value typed there selects it. It applies live through `Shell::select`: a field whose
text changed is read on Enter or blur, and every other field is written again from the result,
except one holding the keyboard that is already typed in, since a field's blur arrives after the
next one took the keyboard. Tab and Shift+Tab go round its six fields, bound in its own context
(`NextField`, `PreviousField`) because the window's Tab would walk out of it.
Up and Down step the focused field, here and in the badge input, by one of the last digit it
shows, ten with Shift, and a sample index by one sample (`Scale::time_step`,
`Scale::band_step`); a changed text is read first. It closes as the analysis hint does, which the
owner chose over a live plot: its backdrop freezes the plot and a press outside closes it keeping
the selection, and so do the title bar, the frame's edges and the application menu
(`Shell::close_hints`). Opening it closes the analysis hint and opening that closes it, keeping the values, so only one
frozen backdrop stands at a time. A press on the title bar that closed it does not reopen it from the lit
segment (`Shell::press_closed_selection`). `SelectionPanel::follow` still takes up a selection
changed while it is open. Enter keeps the values and closes it once every changed field reads,
since the panel handles `Confirm` before the popover; Escape restores the selection as it stood before the
panel or its segment changed it (`Shell::selection_opening`). Ctrl+O closes it keeping the selection
(`Shell::close_selection_hint`), and it selects only in the document it opened on, closing when
another one opens. A badge input that loses the keyboard to nothing, as when the window is left,
hands it back to the plot. A typed selection is an ordinary selection, never an edit.

## Ruler marks and grid visibility (#71, #72)

GUI time labels request `LabelMetrics::keep_edge_marks`: spacing is selected using
readable labels as before, then edge-clipped labels become empty strings while
their valid marks remain. Default CLI layout is unchanged. Both GUI ruler ticks
extend six logical pixels. Right-hand ruler labels start nine pixels after the tick.
Bottom-ruler labels in both orientations (#145, #148) start four clear pixels after
their tick: time in horizontal orientation, frequency in vertical. Their ink sits at
least six pixels under the ruler line and at least eight above the band's bottom edge
(`BOTTOM_LABEL_FOOT`), which keeps them optically clear of the status bar. An Alt
badge wraps the ink with 3 pixels, so it clears the ruler line by at least 3. `session.show_grid`
(version 8, default true for older files) controls only grid strokes; View > Show
grid saves the choice and notifies the UI without requesting analysis.

## Spectrogram orientation (#82)

`orientation::Mode` maps physical time/frequency fractions and rectangles to the
screen; spectral grids retain their original time-column/frequency-row order.
Horizontal mode is the default. Vertical mode puts time downwards, frequency to
the right and the independent minimap on the left. Both the vertical spectrum
and minimap meet the panel top without an outer inset. Uploaded BGRA pixels rotate
clockwise in vertical mode, including padded deep-preview strips; held pictures
and backdrop coverage use the same orientation mapping. GPU retirement still
waits two frame callbacks. Switching mode reuploads retained images and requests
a cached display size, preserving physical ranges unless the existing time ULP
floor requires expansion for a longer time axis. That exceptional range change
restarts analysis just as an ordinary resize does.
`PlotSize` always counts time columns and frequency rows; axes, hints, gestures,
minimap rebinning and the minimap height use the corresponding screen dimension.
The mode segments and Ctrl+T share one orientation action and persist session version 9.
Its shortcut hint resolves the registered binding; plot bindings match only while
the plot surface itself has focus, so they cannot toggle behind a popup or input. Arrow bindings use Horizontal/Vertical
key contexts so panning follows the screen; named time/frequency zoom shortcuts
retain their axes. In vertical mode time labels occupy the right gutter, frequency
labels the bottom row, with time units at the top of the right gutter and frequency
units at bottom-right. The time ruler fits numeric ink between the top time-unit and bottom frequency-unit
captions with four logical pixels of clearance, including the painted half-pixel
center offset, without moving the spectrum top or removing grid marks. Unit hitboxes retain an
arrow cursor and exclude pan/zoom gestures; the time unit retains its context menu. The grid and time-format settings apply
to both layouts.

A middle-button or Space+left drag on the spectrum pans both physical axes, including frequency-only zoom when
time remains fitted. Rulers and minimap constrain dragging to their own axis.
A pointer event updates both viewports before submitting one display request.
Wheel handling uses the dominant nonzero delta component because Linux GPUI
backends remap Shift+wheel to horizontal deltas; modifiers still select the axis.

Symbolic zoom keys are normalized by a keystroke interceptor that each `PlotView`
owns and drops with itself. It runs before GPUI binding match, which is the only hook
that early, and acts only when its window is the event's window and its own plot
surface is exactly focused; otherwise it neither matches nor consumes anything. Grid
and orientation keys are ordinary bindings. It combines the keystroke Shift flag with the
window's physical Shift state, which Linux symbol normalization otherwise loses.
Ctrl plus/minus targets time; adding Shift targets frequency, including
underscore and keypad aliases; a top-row `=` keystroke is accepted silently as
the same physical key as `+`, and the keycaps show the four zoom commands
uniformly as Ctrl+Plus, Ctrl+Shift+Plus, Ctrl+Minus, Ctrl+Shift+Minus
(`shortcuts::zoom_keycap`). Ctrl+Shift+0 fits frequency; the interceptor handles
both `0` and the US shifted `)` with physical Shift state, leaving unshifted Ctrl+0
to the time-fit binding. Named menu actions retain their explicit axes.
Ctrl+Shift+Up/Down are not zoom bindings; arrow pan bindings stay unchanged.

## Waveform and axis layout

The waveform is 3 rem (48 logical pixels with the default font) by default, with a 1-logical-pixel theme-border separator spanning only the waveform plot. The time and frequency ruler baselines use the same colour as their ticks; the spectrum starts directly below the waveform separator without a top inset. The minimap is not resizable (#132): `[panels].minimap_size` in `argand.toml` fixes its height, or its width in vertical orientation, as `"<number> rem"` (1 to 20, following the interface font) or `"<number> px"` (16 to 320 logical pixels), default `"3 rem"`. The value must be a quoted string in TOML. An unusable value is logged and replaced by the default alone. A `waveform_fraction` left in an older session is ignored and dropped on the next save. `panels.rs` parses the size and computes it on device pixels, while `waveform.rs` draws one merged real or I/Q envelope without grid lines, a zero-axis line or labels, using the exact horizontal geometry measured for the spectrogram. `WaveformEnvelope::pixel_spans` shares channel merging, pixel rounding and continuity with `aspec`; The minimap amplitude scale comes from its full-capture peak, independently of spectral dynamic range. Its envelope survives spectrum replacement. The old `[panels].waveform_fraction` configuration remains readable for compatibility but does not size the panel. Progressive waveform integration is implemented by #29; synchronized time navigation is implemented by #30.

The window measures its axis labels through `argand_core::axis::LabelMeasure`, implemented over GPUI's text system. That is the second implementation of the trait, `aspec`'s ab_glyph one being the first, and it is why the trait exists. The GUI sizes the right ruler's gutter from the labels it actually places plus every unit caption (#148). `PlotView` holds the widest gutter measured (`axes::Held::gutter`), so zooming in, panning and zooming the other axis never narrow it. `PlotView::view_intent` releases the hold when the axis shown on the right zooms out or is fitted, so the ruler refits its new labels (it can still widen when a format or unit grows). A new file, an orientation change and a time-format change with time on the right also release it. The GUI starts time labels after their ticks; shared axis layout accounts for that anchor while retaining centered labels by default for CLI rendering. Numeric label centering uses the shaped line's baseline metrics. The axis layout keeps a 4-logical-pixel outer margin at the sides and bottom and meets the minimap separator at the top. The frequency unit occupies the right gutter beside the minimap, with the bottom of its text aligned to the top of the frequency ruler; only time labels reserve a full text row outside the plot height.
