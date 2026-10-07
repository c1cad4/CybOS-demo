//! Web orchestration for the RobotCYB planner.

use crate::CybOs;
use crate::network::web_urls;

pub(crate) enum WebToolOutcome {
    Continue(String),
    Answer(String),
}

impl CybOs {
    pub(crate) fn direct_web_answer(
        &self,
        conversation: &str,
        web_query: &str,
    ) -> String {
        let search_result = self.tool_web_search(web_query);
        let urls = web_urls::web_urls_from_result(&search_result);

        if urls.is_empty() {
            return "Веб-поиск не нашёл подходящих источников.".into();
        }

        for url in urls {
            let fetched = self.tool_web_fetch(&url);

            if !fetched.starts_with("WEB SOURCE\n") {
                continue;
            }

            return self.web_answer_from_source(conversation, &fetched);
        }

        "Поиск выполнился, но cybOS не смог открыть источник. Непроверенная информация не была выдана как факт."
            .into()
    }

    pub(crate) fn handle_web_tool(
        &self,
        tool: &str,
        arguments: &str,
        conversation: &str,
    ) -> WebToolOutcome {
        match tool {
            "web_search" => {
                let result = self.tool_web_search(arguments);
                let urls = web_urls::web_urls_from_result(&result);

                if urls.is_empty() {
                    return WebToolOutcome::Answer(
                        "Поиск выполнился, но источник не удалось открыть. Непроверенная информация не была выдана как факт."
                            .into(),
                    );
                }

                for url in urls {
                    let fetched = self.tool_web_fetch(&url);

                    if fetched.starts_with("WEB SOURCE\n") {
                        return WebToolOutcome::Answer(
                            self.web_answer_from_source(conversation, &fetched),
                        );
                    }
                }

                WebToolOutcome::Answer(
                    "Поиск выполнился, но источник не удалось открыть. Непроверенная информация не была выдана как факт."
                        .into(),
                )
            }

            "web_fetch" => {
                let fetched = self.tool_web_fetch(arguments);

                if fetched.starts_with("WEB SOURCE\n") {
                    return WebToolOutcome::Answer(
                        self.web_answer_from_source(conversation, &fetched),
                    );
                }

                WebToolOutcome::Continue(fetched)
            }

            _ => WebToolOutcome::Continue(String::new()),
        }
    }
}
