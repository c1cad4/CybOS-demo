//! Learning validation and user-message processing.

use crate::CybOs;
use crate::models::{LearningFact, LearningResponse, Memory};
use chrono::Local;
use serde_json::json;
use uuid::Uuid;

impl CybOs {
    pub(crate) fn valid_learning_fact(fact: &LearningFact) -> bool {
        !fact.subject.trim().is_empty()
            && !fact.subject_label.trim().is_empty()
            && !fact.subject_kind.trim().is_empty()
            && !fact.relation.trim().is_empty()
            && !fact.object.trim().is_empty()
            && !fact.object_label.trim().is_empty()
            && !fact.object_kind.trim().is_empty()
            && fact.relation.len() <= 80
            && fact.subject_kind.len() <= 40
            && fact.object_kind.len() <= 40
    }


    pub(crate) fn learn_from_user_message(&mut self, q: &str) {
        let text = q.trim();


        if text.is_empty() {
            return;
        }


        let url = "http://127.0.0.1:8080/v1/chat/completions";


        let system_prompt = r#"
    You are the cybOS Knowledge Extractor.


    Extract only explicit factual knowledge from the user's message.


    Return ONLY valid JSON:


    {
      "facts": [
    {
      "subject": "stable-id",
      "subject_label": "human readable label",
      "subject_kind": "ENTITY_KIND",
      "relation": "relation_name",
      "object": "stable-id-or-value",
      "object_label": "human readable value",
      "object_kind": "ENTITY_KIND"
    }
      ]
    }


    Rules:
    - Extract only facts explicitly stated by the user.
    - Never invent facts.
    - Never infer unstated facts.
    - IDs must be lowercase ASCII using hyphens.
    - Use concise relations such as located_at, has_frames, contains.
    - Numeric values use object_kind VALUE.
    - Places use object_kind PLACE.
    - Hives use subject_kind BEE.
    - If there is no factual knowledge, return {"facts":[]}.
    - Return JSON only.
    "#;


        let payload = json!({
            "model": "mlx-community/Qwen3.5-9B-MLX-4bit",
            "messages": [
                {
                    "role": "system",
                    "content": system_prompt
                },
                {
                    "role": "user",
                    "content": text
                }
            ],
            "max_tokens": 1200,
            "temperature": 0.0,
            "chat_template_kwargs": {
                "enable_thinking": false
            }
        });


        let response = match ureq::post(url)
            .header("Content-Type", "application/json")
            .send_json(&payload)
        {
            Ok(resp) => resp,
            Err(_) => return,
        };


        let body = match response.into_body().read_to_string() {
            Ok(body) => body,
            Err(_) => return,
        };


        let value: serde_json::Value = match serde_json::from_str(&body) {
            Ok(value) => value,
            Err(_) => return,
        };


        let message = &value["choices"][0]["message"];


        let content = message["content"]
            .as_str()
            .filter(|v| !v.trim().is_empty())
            .or_else(|| {
                message["reasoning"]
                    .as_str()
                    .filter(|v| !v.trim().is_empty())
            })
            .or_else(|| {
                value["choices"][0]["text"]
                    .as_str()
                    .filter(|v| !v.trim().is_empty())
            });


        let Some(content) = content else {
            return;
        };


        let cleaned = content
            .trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();


        let json_text = if serde_json::from_str::<LearningResponse>(cleaned).is_ok() {
            cleaned.to_string()
        } else {
            let start = match cleaned.find('{') {
                Some(v) => v,
                None => return,
            };


            let end = match cleaned.rfind('}') {
                Some(v) => v,
                None => return,
            };


            cleaned[start..=end].to_string()
        };


        let learning: LearningResponse = match serde_json::from_str(&json_text) {
            Ok(value) => value,
            Err(_) => return,
        };


        let mut accepted = 0usize;


        for fact in learning.facts {
            if !Self::valid_learning_fact(&fact) {
                continue;
            }


            let Some(subject_id) = Self::normalize_learning_id(&fact.subject) else {
                continue;
            };


            let Some(object_id) = Self::normalize_learning_id(&fact.object) else {
                continue;
            };


            if subject_id == object_id {
                continue;
            }


            let subject_kind = fact.subject_kind.trim().to_uppercase();
            let object_kind = fact.object_kind.trim().to_uppercase();
            let relation = fact.relation.trim().to_lowercase();


            if !self.nodes.iter().any(|n| n.id == subject_id) {
                self.add_graph_node(&subject_id, fact.subject_label.trim(), &subject_kind);
            }


            if !self.nodes.iter().any(|n| n.id == object_id) {
                self.add_graph_node(&object_id, fact.object_label.trim(), &object_kind);
            }


            if self.add_graph_link(&subject_id, &object_id, &relation) {
                accepted += 1;
            }
        }


        if accepted > 0 {
            let memory = Memory {
                id: Uuid::new_v4().to_string(),
                time: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                text: text.to_string(),
                source: "USER".into(),
                importance: 1.0,
            };


            self.store.add_memory(&memory);


            self.add_event("MEMORY", format!("User knowledge stored: {}", text));


            self.add_event(
                "LEARNING",
                format!(
                    "Accepted {} graph relationship(s) from user message",
                    accepted
                ),
            );
        }
    }
}
