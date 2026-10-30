//! State transitions shared by the global search and quick filter UI.
use crate::{App, ModalState};
use agmux_proto::{IpcMessage, SearchNotesRequest, SearchNotesResponse};

impl App {
    /// Whether a session is visible under both sidebar filters.
    pub fn session_visible(&self, index: usize) -> bool {
        self.sessions.get(index).is_some_and(|session| {
            (self.metadata_for(index).archived == self.show_archived)
                && (!self.attention_filter || !matches!(session.state.as_str(), "IDLE" | "DEAD"))
                && self
                    .filter_ids
                    .as_ref()
                    .is_none_or(|ids| ids.contains(&session.session_id))
        })
    }

    /// Keep the selected session inside the visible set when possible.
    pub fn align_selection(&mut self) {
        if !self.session_visible(self.selected_index) {
            self.selected_index = (0..self.sessions.len())
                .find(|&i| self.session_visible(i))
                .unwrap_or(usize::MAX);
        }
    }

    /// Request the current quick filter, or clear it when empty.
    pub fn refresh_filter(&mut self) {
        if self.filter_query.is_empty() {
            self.filter_ids = None;
            self.align_selection();
        } else {
            self.pending_message =
                Some(IpcMessage::SearchNotesRequest(SearchNotesRequest::Filter {
                    query: self.filter_query.clone(),
                }));
        }
    }

    /// Apply only responses that still match the current input and page.
    pub fn apply_search_notes_response(&mut self, response: SearchNotesResponse) {
        match response.request {
            SearchNotesRequest::Search { query } => {
                if let ModalState::GlobalSearch {
                    query: current,
                    selected,
                    hits,
                    error,
                } = &mut self.modal
                {
                    if *current == query {
                        let previous = hits
                            .get(*selected)
                            .map(|hit| (hit.session_id.clone(), hit.page, hit.line_no));
                        *selected = previous
                            .and_then(|(session_id, page, line_no)| {
                                response.hits.iter().position(|hit| {
                                    hit.session_id == session_id
                                        && hit.page == page
                                        && hit.line_no == line_no
                                })
                            })
                            .unwrap_or(0);
                        *hits = response.hits;
                        *error = response.error;
                    }
                }
            }
            SearchNotesRequest::Filter { query } if query == self.filter_query => {
                self.feature_error = response.error;
                self.filter_ids = Some(response.session_ids);
                self.align_selection();
            }
            SearchNotesRequest::Get { session_id, page } => {
                if let ModalState::Notes {
                    session_id: current,
                    page: current_page,
                    content,
                    loaded,
                } = &mut self.modal
                {
                    if *current == session_id && *current_page == page && !*loaded {
                        if response.error.is_none() {
                            *content = response
                                .pages
                                .into_iter()
                                .next()
                                .map(|(_, value)| value)
                                .unwrap_or_default();
                            *loaded = true;
                        }
                        self.feature_error = response.error;
                    }
                }
            }
            SearchNotesRequest::Set { .. } => self.feature_error = response.error,
            _ => {}
        }
    }
}
