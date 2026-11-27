//! Tabs and split panes: create, split, focus.

use crate::*;

pub(crate) fn first_id(node: &Node) -> u64 {
    match node {
        Node::Pane(p) => p.id,
        Node::Split { first, .. } => first_id(first),
    }
}

impl TerminalApp {
    pub(crate) fn new_pane_with_shell(&mut self, shell: Shell) -> Pane {
        let id = self.next_id;
        self.next_id += 1;
        Pane {
            remote: None,
            id,
            shell,
            directory: self
                .workspace
                .inherited_directory(self.focused, std::path::Path::new(&self.directory)),
        }
    }

    pub(crate) fn add_tab(&mut self, ctx: &egui::Context) {
        self.add_tab_with_shell(ctx, self.workspace.default_shell);
    }

    pub(crate) fn add_tab_with_shell(&mut self, ctx: &egui::Context, shell: Shell) {
        if self.workspace.tabs.len() >= 32 || self.sessions.len() >= 64 {
            self.status = tr("Limit: 32 Tabs / 64 Bereiche").into();
            return;
        }
        let pane = self.new_pane_with_shell(shell);
        self.editor.active = false;
        self.start(ctx, &pane);
        self.focused = pane.id;
        self.workspace.tabs.push(Tab {
            name: pane.shell.label().into(),
            root: Node::Pane(pane),
        });
        self.workspace.active_tab = self.workspace.tabs.len() - 1;
    }

    pub(crate) fn split(&mut self, ctx: &egui::Context, axis: Axis) {
        self.split_with_shell(ctx, axis, self.workspace.default_shell);
    }

    pub(crate) fn split_with_shell(&mut self, ctx: &egui::Context, axis: Axis, shell: Shell) {
        if self.editor.active {
            return;
        }
        if self.sessions.len() >= 64 {
            self.status = tr("Maximal 64 Bereiche").into();
            return;
        }
        let pane = self.new_pane_with_shell(shell);
        if let Some(tab) = self.workspace.tabs.get_mut(self.workspace.active_tab) {
            if tab.root.split(self.focused, axis, pane.clone()) {
                self.start(ctx, &pane);
                self.focused = pane.id;
            }
        }
    }

    pub(crate) fn focus_first(&mut self) {
        self.editor.active = false;
        let mut panes = vec![];
        if let Some(tab) = self.workspace.tabs.get(self.workspace.active_tab) {
            tab.root.panes(&mut panes);
        }
        self.focused = panes.first().map(|p| p.id).unwrap_or(0);
    }

    pub(crate) fn cycle_pane(&mut self) {
        let mut panes = vec![];
        if let Some(tab) = self.workspace.tabs.get(self.workspace.active_tab) {
            tab.root.panes(&mut panes);
        }
        if !panes.is_empty() {
            let index = panes.iter().position(|p| p.id == self.focused).unwrap_or(0);
            self.focused = panes[(index + 1) % panes.len()].id;
        }
    }
}
