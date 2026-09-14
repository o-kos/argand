//! Toolkit-independent navigation of a cascading application menu.

#[derive(Clone)]
pub struct Item<T> {
    pub label: String,
    pub checked: bool,
    pub enabled: bool,
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
            kind: Kind::Command(command),
        }
    }

    pub fn branch(label: impl Into<String>, items: Vec<Self>) -> Self {
        Self {
            label: label.into(),
            checked: false,
            enabled: !items.is_empty(),
            kind: Kind::Branch(items),
        }
    }

    pub fn separator() -> Self {
        Self {
            label: String::new(),
            checked: false,
            enabled: false,
            kind: Kind::Separator,
        }
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
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
