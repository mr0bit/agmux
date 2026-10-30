//! Persisted workspace organization and identity-preserving sidebar ordering.
use crate::App;
use agmux_proto::{IpcMessage, WorkspaceMetadata, WorkspaceRequest, WorkspaceResponse};

impl App {
    pub fn metadata_for(&self, index: usize) -> WorkspaceMetadata {
        let Some(session) = self.sessions.get(index) else {
            return WorkspaceMetadata::default();
        };
        self.workspace
            .get(&session.session_id)
            .cloned()
            .unwrap_or_else(|| WorkspaceMetadata {
                project: session.cwd.clone(),
                archived: session.state == "ARCHIVED",
                ..WorkspaceMetadata::default()
            })
    }

    /// Session indices in the same order as rendered, without duplicating favorites.
    pub fn workspace_order(&self) -> Vec<usize> {
        let mut indices: Vec<_> = (0..self.sessions.len())
            .filter(|&i| self.session_visible(i))
            .collect();
        indices.sort_by_cached_key(|&i| {
            let meta = self.metadata_for(i);
            (!meta.favorite, meta.project, meta.group, i)
        });
        indices
    }

    pub fn update_workspace(&mut self, metadata: WorkspaceMetadata) {
        if let Some(session) = self.sessions.get(self.selected_index) {
            let session_id = session.session_id.clone();
            self.workspace.insert(session_id.clone(), metadata.clone());
            self.pending_message = Some(IpcMessage::WorkspaceRequest(WorkspaceRequest::Set {
                session_id,
                metadata,
            }));
            self.align_selection();
        }
    }

    pub fn apply_workspace(&mut self, response: WorkspaceResponse) {
        if let Some(error) = response.error {
            self.feature_error = Some(error);
            return;
        }
        let selected = self
            .sessions
            .get(self.selected_index)
            .map(|s| s.session_id.clone());
        let pinned = self
            .pinned_session_index
            .and_then(|i| self.sessions.get(i))
            .map(|s| s.session_id.clone());
        self.sessions = response.sessions;
        self.workspace = response.metadata;
        self.subtabs
            .retain(|id| self.sessions.iter().any(|s| &s.session_id == id));
        for session in &self.sessions {
            if session.session_id.contains(":fork-") && !self.subtabs.contains(&session.session_id)
            {
                self.subtabs.push(session.session_id.clone());
            }
        }
        self.active_subtab_idx = self
            .active_subtab_idx
            .min(self.subtabs.len().saturating_sub(1));
        self.selected_index = selected
            .and_then(|id| self.sessions.iter().position(|s| s.session_id == id))
            .unwrap_or(0);
        self.pinned_session_index =
            pinned.and_then(|id| self.sessions.iter().position(|s| s.session_id == id));
        self.align_selection();
    }
}
