//! Toolkit-independent navigation of a cascading application menu.

#[derive(Clone)]
pub struct Item<T> {
    pub label: String,
    pub checked: bool,
    pub enabled: bool,
    /// The quick digit the row promises; the renderer shows it as a keycap
    /// and the matching key activates the row while its level is open.
    pub number: Option<u8>,
    pub kind: Kind<T>,
}

#[derive(Clone)]
pub enum Kind<T> {
    Command(T),
    Branch(Vec<Item<T>>),
    Separator,
}

impl<T> Item<T> {
    pub fn command(label: impl Into<String>, command: T) -> Self {
        Self {
            label: label.into(),
            checked: false,
            enabled: true,
            number: None,
            kind: Kind::Command(command),
        }
    }

    pub fn branch(label: impl Into<String>, items: Vec<Self>) -> Self {
        Self {
            label: label.into(),
            checked: false,
            enabled: !items.is_empty(),
            number: None,
            kind: Kind::Branch(items),
        }
    }

    pub fn separator() -> Self {
        Self {
            label: String::new(),
            checked: false,
            enabled: false,
            number: None,
            kind: Kind::Separator,
        }
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    pub fn numbered(mut self, number: u8) -> Self {
        self.number = Some(number);
        self
    }
}

pub struct Menu<T> {
    items: Vec<Item<T>>,
    selected: Vec<Option<usize>>,
}

#[derive(Debug, PartialEq)]
pub enum Effect<T> {
    None,
    Activate(T),
    Dismiss,
}

impl<T: Clone> Menu<T> {
    pub fn new(items: Vec<Item<T>>) -> Self {
        Self {
            items,
            selected: vec![None],
        }
    }

    pub fn depth(&self) -> usize {
        self.selected.len()
    }

    pub fn selected(&self, level: usize) -> Option<usize> {
        self.selected.get(level).copied().flatten()
    }

    pub fn items(&self, level: usize) -> &[Item<T>] {
        let mut items = self.items.as_slice();
        for index in self.selected.iter().take(level) {
            let Some(Item {
                kind: Kind::Branch(children),
                ..
            }) = index.and_then(|i| items.get(i))
            else {
                return &[];
            };
            items = children;
        }
        items
    }

    pub fn select(&mut self, level: usize, index: usize) {
        if level >= self.depth() {
            return;
        }
        self.selected.truncate(level + 1);
        self.selected[level] = self
            .items(level)
            .get(index)
            .filter(|item| item.enabled)
            .map(|_| index);
    }

    pub fn hover(&mut self, level: usize, index: usize) {
        if self.selected(level) == Some(index) && self.depth() > level + 1 {
            return;
        }
        self.select(level, index);
        self.expand(level);
    }

    fn expand(&mut self, level: usize) -> bool {
        let branch = self.selected(level).and_then(|i| self.items(level).get(i));
        if !matches!(
            branch,
            Some(Item {
                kind: Kind::Branch(_),
                enabled: true,
                ..
            })
        ) {
            return false;
        }
        self.selected.truncate(level + 1);
        self.selected.push(None);
        true
    }

    pub fn step(&mut self, forward: bool) {
        let level = self.depth() - 1;
        let enabled: Vec<_> = self
            .items(level)
            .iter()
            .enumerate()
            .filter_map(|(i, item)| item.enabled.then_some(i))
            .collect();
        if enabled.is_empty() {
            return;
        }
        let current = enabled
            .iter()
            .position(|&i| Some(i) == self.selected(level));
        let next = match (current, forward) {
            (Some(i), true) => (i + 1) % enabled.len(),
            (Some(i), false) => (i + enabled.len() - 1) % enabled.len(),
            (None, true) => 0,
            (None, false) => enabled.len() - 1,
        };
        self.select(level, enabled[next]);
    }

    pub fn edge(&mut self, last: bool) {
        let level = self.depth() - 1;
        self.selected[level] = None;
        self.step(!last);
    }

    pub fn back(&mut self) -> Effect<T> {
        if self.depth() == 1 {
            return Effect::Dismiss;
        }
        self.selected.pop();
        Effect::None
    }

    pub fn enter(&mut self, execute: bool) -> Effect<T> {
        let level = self.depth() - 1;
        if self.expand(level) {
            self.step(true);
            return Effect::None;
        }
        match self.selected(level).and_then(|i| self.items(level).get(i)) {
            Some(Item {
                kind: Kind::Command(command),
                ..
            }) if execute => Effect::Activate(command.clone()),
            _ => Effect::None,
        }
    }

    /// Activate the open level's row carrying `number`, the way the keycap
    /// on that row promises. Only enabled commands qualify: a missing,
    /// disabled, unnumbered or non-command digit does nothing and leaves
    /// the selection alone.
    pub fn activate_numbered(&mut self, number: u8, execute: bool) -> Effect<T> {
        let level = self.depth() - 1;
        let Some(index) = self.items(level).iter().position(|item| {
            item.number == Some(number) && item.enabled && matches!(item.kind, Kind::Command(_))
        }) else {
            return Effect::None;
        };
        self.select(level, index);
        if self.selected(level) == Some(index) {
            self.enter(execute)
        } else {
            Effect::None
        }
    }
}

pub fn file_items<T>(open: Item<T>, recent: Vec<Item<T>>, settings: Item<T>) -> Vec<Item<T>> {
    let mut items = vec![open, Item::separator()];
    if !recent.is_empty() {
        items.extend(recent);
        items.push(Item::separator());
    }
    items.push(settings);
    items
}

/// Keep a popup inside the viewport, flipping children to the left when needed.
pub fn place(parent: [f32; 4], desired: [f32; 2], viewport: [f32; 2], child: bool) -> [f32; 4] {
    let [vw, vh] = viewport.map(|v| v.max(1.));
    let mut width = desired[0].min((vw - 8.).max(1.));
    let height = desired[1].min((vh - 8.).max(1.));
    let [x, y, w, h] = parent;
    let left = if child && x + w + width > vw - 4. {
        // Keep a strip of the parent reachable when a wide child overlaps it.
        width = width.min((x + w - 36.).max(1.));
        x - width
    } else if child {
        x + w
    } else {
        x
    };
    let top = if child { y } else { y + h };
    [
        left.clamp(4., (vw - width - 4.).max(4.)),
        top.clamp(4., (vh - height - 4.).max(4.)),
        width,
        height,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn menu() -> Menu<u8> {
        Menu::new(vec![
            Item::branch(
                "File",
                vec![
                    Item::command("Open", 1),
                    Item::branch("Recent", vec![Item::command("Capture", 2)]),
                ],
            ),
            Item::branch(
                "View",
                vec![
                    Item::command("Grid", 3),
                    Item::separator(),
                    Item::branch("Empty", vec![]),
                    Item::command("Orientation", 4),
                ],
            ),
        ])
    }

    #[test]
    fn escape_closes_one_level_and_preserves_parent_selection() {
        let mut menu = menu();
        menu.hover(0, 0);
        menu.hover(1, 1);
        assert_eq!(menu.depth(), 3);
        assert_eq!(menu.back(), Effect::None);
        assert_eq!(menu.depth(), 2);
        assert_eq!(menu.selected(1), Some(1));
        assert_eq!(menu.back(), Effect::None);
        assert_eq!(menu.back(), Effect::Dismiss);
    }

    #[test]
    fn hover_switches_siblings_and_discards_old_descendants() {
        let mut menu = menu();
        menu.hover(0, 0);
        menu.hover(1, 1);
        menu.hover(0, 1);
        assert_eq!(menu.depth(), 2);
        menu.step(true);
        assert_eq!(menu.enter(true), Effect::Activate(3));
    }

    #[test]
    fn keyboard_skips_unavailable_rows_wraps_and_opens_branches() {
        let mut menu = menu();
        menu.edge(true);
        assert_eq!(menu.enter(false), Effect::None);
        assert_eq!(menu.selected(1), Some(0));
        menu.step(false);
        assert_eq!(menu.selected(1), Some(3));
        menu.step(true);
        assert_eq!(menu.selected(1), Some(0));
        assert_eq!(menu.enter(false), Effect::None);
        assert_eq!(menu.enter(true), Effect::Activate(3));
    }

    #[test]
    fn disabled_hover_cannot_activate_previous_command() {
        let mut menu = menu();
        menu.hover(0, 1);
        menu.hover(1, 0);
        menu.hover(1, 2);
        assert_eq!(menu.enter(true), Effect::None);
    }

    #[test]
    fn recent_rows_are_inline_and_their_separators_collapse_when_empty() {
        let items = file_items(
            Item::command("Open file...", 1),
            vec![Item::command("Capture", 2), Item::command("Capture", 5)],
            Item::command("Settings", 3),
        );
        let kinds: Vec<&str> = items
            .iter()
            .map(|item| match item.kind {
                Kind::Command(_) => "command",
                Kind::Branch(_) => "branch",
                Kind::Separator => "separator",
            })
            .collect();
        assert_eq!(
            kinds,
            vec![
                "command",
                "separator",
                "command",
                "command",
                "separator",
                "command"
            ]
        );
        let empty = file_items(
            Item::command("Open file...", 1),
            Vec::new(),
            Item::command("Settings", 3),
        );
        assert_eq!(empty.len(), 3);
        assert!(matches!(empty[1].kind, Kind::Separator));
    }

    #[test]
    fn numbered_recent_rows_activate_by_their_digit() {
        let mut menu = Menu::new(vec![Item::branch(
            "File",
            file_items(
                Item::command("Open file...", 1),
                vec![
                    Item::command("a", 2).numbered(1),
                    Item::command("b", 3).numbered(2),
                ],
                Item::command("Settings", 4),
            ),
        )]);
        menu.hover(0, 0);
        assert_eq!(menu.depth(), 2);
        assert_eq!(menu.activate_numbered(2, true), Effect::Activate(3));
        assert_eq!(menu.selected(1), Some(3));
    }

    #[test]
    fn missing_disabled_and_root_level_digits_do_nothing() {
        let mut disabled = Item::command("b", 3).numbered(2);
        disabled.enabled = false;
        let mut menu = Menu::new(vec![Item::branch(
            "File",
            file_items(
                Item::command("Open file...", 1),
                vec![Item::command("a", 2).numbered(1), disabled],
                Item::command("Settings", 4),
            ),
        )]);
        // Digits mean nothing before the branch is open.
        assert_eq!(menu.activate_numbered(1, true), Effect::None);
        menu.hover(0, 0);
        // An unknown digit and a disabled row both stay inert.
        assert_eq!(menu.activate_numbered(9, true), Effect::None);
        assert_eq!(menu.activate_numbered(2, true), Effect::None);
        assert_eq!(menu.selected(1), None);
    }

    #[test]
    fn digits_never_touch_separators_branches_or_the_selection() {
        let numbered_branch = Item::branch("Sub", vec![Item::command("leaf", 5)]).numbered(1);
        let numbered_separator = Item::separator().numbered(2);
        let mut numbered_disabled = Item::command("b", 3).numbered(3);
        numbered_disabled.enabled = false;
        let mut menu = Menu::new(vec![Item::branch(
            "File",
            vec![
                Item::command("Open file...", 4),
                numbered_branch,
                numbered_separator,
                numbered_disabled,
            ],
        )]);
        menu.hover(0, 0);
        menu.select(1, 0);
        // A digit on a branch, a separator or a disabled row changes
        // nothing: no expansion, no selection reset.
        for digit in [1, 2, 3] {
            assert_eq!(menu.activate_numbered(digit, true), Effect::None);
            assert_eq!(menu.depth(), 2);
            assert_eq!(menu.selected(1), Some(0));
        }
    }

    #[test]
    fn wide_child_leaves_parent_branch_reachable() {
        let parent = [170., 90., 190., 26.];
        let [x, _, width, _] = place(parent, [420., 200.], [640., 400.], true);
        assert!(x + width <= parent[0] + parent[2] - 32.);
        assert!(width >= 300.);
    }

    #[test]
    fn popups_flip_and_fit_small_viewports() {
        assert_eq!(
            place([400., 20., 120., 30.], [250., 450.], [640., 400.], true),
            [150., 4., 250., 392.]
        );
        assert_eq!(
            place([10., 15., 30., 30.], [160., 60.], [640., 400.], false),
            [10., 45., 160., 60.]
        );
        let [x, y, w, h] = place([0., 0., 0., 0.], [400., 400.], [100., 100.], true);
        assert!(x >= 0. && y >= 0. && x + w <= 100. && y + h <= 100.);
    }
}
