use crate::app::{CybOs, Page};
use std::time::Instant;

impl CybOs {



    pub(crate) fn notify(&mut self, text: impl Into<String>) {
        self.toast = Some((text.into(), Instant::now()));
        self.status = "LOCAL-FIRST · READY".into();
    }

    pub(crate) fn go(&mut self, page: Page) {
        if self.page != page {
            self.page = page;
            self.notify(format!("OPENED {}", page.title()));
        }
    }

    pub(crate) fn apply_search(&mut self) {
        let q = self.search.trim().to_lowercase();
        if q.is_empty() {
            self.notify("TYPE A PAGE, TOKEN, OR SYSTEM NAME");
            return;
        }
        let pages = [
            Page::Dashboard,
            Page::Robot,
            Page::Farm,
            Page::Chat,
            Page::Graph,
            Page::Brain,
            Page::Network,
            Page::Assets,
            Page::Cameras,
            Page::System,
        ];
        if let Some(page) = pages.into_iter().find(|p| p.matches_query(&q)) {
            self.go(page);
            return;
        }
        if q.contains("event") || q.contains("log") {
            self.go(Page::Dashboard);
            return;
        }
        self.notify(format!("NO MATCH FOR “{}”", self.search.trim()));
    }

    pub(crate) fn add_event(&mut self, kind: &str, text: impl Into<String>) {
        let text = text.into();
        self.store.add_event(kind, &text);
        self.events = self.store.events();
    }
}
