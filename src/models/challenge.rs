use crate::models::scenario::Scenario;
use std::collections::BTreeMap;

#[derive(Clone)]
pub struct Challenge {
    pub id: i64,
    pub name: String,
    pub code: String,
    pub description: Option<String>,
    pub scenario: Option<Scenario>,
}

pub fn group_by_scenario(challenges: Vec<Challenge>) -> BTreeMap<Scenario, Vec<Challenge>> {
    let mut scenarios: BTreeMap<Scenario, Vec<Challenge>> = BTreeMap::new();

    for challenge in challenges.into_iter() {
        if let Some(scenario) = challenge.scenario.clone() {
            let value = scenarios.entry(scenario).or_insert(Vec::new());
            value.push(challenge);
        }
    }

    scenarios
}
