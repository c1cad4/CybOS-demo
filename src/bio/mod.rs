use chrono::{DateTime, Utc};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BioObservation {
    pub animal_id: String,
    pub kind: String,
    pub value: String,
    pub observed_at: DateTime<Utc>,
    pub source: String,
}

impl BioObservation {
    pub fn validate(&self) -> Result<(), String> {
        if self.animal_id.trim().is_empty() { return Err("animal id is empty".into()); }
        if self.kind.trim().is_empty() { return Err("bio observation kind is empty".into()); }
        if self.value.len() > 4096 { return Err("bio observation value is too large".into()); }
        if self.source.trim().is_empty() { return Err("bio observation source is empty".into()); }
        Ok(())
    }

    pub fn graph_subject(&self) -> String { format!("animal:{}", self.animal_id.trim()) }
    pub fn graph_event(&self) -> String { format!("bio-event:{}", self.observed_at.timestamp_millis()) }
}

#[cfg(test)]
mod tests {
    use super::BioObservation;
    use chrono::Utc;

    #[test]
    fn bio_observation_maps_to_graph_ids() {
        let observation = BioObservation {
            animal_id: "nika".into(),
            kind: "weight".into(),
            value: "42".into(),
            observed_at: Utc::now(),
            source: "farm-sensor".into(),
        };
        assert!(observation.validate().is_ok());
        assert!(observation.graph_subject().starts_with("animal:"));
        assert!(observation.graph_event().starts_with("bio-event:"));
    }
}
