use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct HobScenario {
    pub title: String,
    pub slug: String,
    pub product: String,
    pub number: usize,
    pub quest_cards: Vec<String>,
    pub scenario_cards: Vec<String>,
}

#[derive(Debug)]
pub struct Scenario {
    pub title: String,
    pub set: String,
    pub number: usize,
}

pub async fn fetch() -> Result<Vec<Scenario>, Box<dyn std::error::Error>> {
    let hob_scenarios = reqwest::get("http://hallofbeorn.com/Export/Scenarios")
        .await?
        .json::<Vec<HobScenario>>()
        .await?;
    let mut deluxes = HashMap::new();
    deluxes.insert("Core Set", "Shadows of Mirkwood");
    deluxes.insert("Khazad-dûm", "Dwarrowdelf");
    deluxes.insert("Heirs of Númenor", "Against the Shadow");
    deluxes.insert("The Voice of Isengard", "The Ring-maker");
    deluxes.insert("The Lost Realm", "Angmar Awakened");
    deluxes.insert("The Grey Havens", "Dream-chaser");
    deluxes.insert("The Sands of Harad", "Haradrim");
    deluxes.insert("The Wilds of Rhovanion", "Ered Mithrin");
    deluxes.insert("A Shadow in the East", "Vengeance of Mordor");
    let others = [
        "The Hobbit: Over Hill and Under Hill",
        "The Hobbit: On the Doorstep",
        "The Black Riders",
        "The Road Darkens",
        "The Treason of Saruman",
        "The Land of Shadow",
        "The Flame of the West",
        "The Mountain of Fire",
        "Two-Player Limited Edition Starter",
    ];

    let mut last_deluxe = String::new();
    let scenarios = hob_scenarios
        .into_iter()
        .filter(|s| s.product != "First Age")
        .map(|s| {
            // figure out which set we're in
            if s.product != last_deluxe && deluxes.contains_key(s.product.as_str()) {
                last_deluxe = s.product.clone();
            }

            let set = if s.product.parse::<u32>().is_ok() {
                "Standalone Scenarios".to_string()
            } else if others.iter().find(|&&o| o == s.product).is_some() {
                s.product.clone()
            } else if s.product != last_deluxe {
                deluxes.get(last_deluxe.as_str()).unwrap().to_string()
            } else {
                last_deluxe.clone()
            };

            Scenario {
                title: s.title,
                set,
                number: s.number,
            }
        })
        .collect();

    Ok(scenarios)
}
