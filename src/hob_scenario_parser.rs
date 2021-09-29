use serde::Deserialize;
use std::collections::HashMap;

const FIRST_AGE_PRODUCTS: &[&str] = &[
    "First Age",
    "Trial Upon the Marches",
    "Among the Outlaws",
    "The Betrayal of Mîm",
    "The Fall of Nargothrond",
];

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct HobScenario {
    pub title: String,
    pub slug: String,
    pub product: String,
    pub number: i16,
    pub quest_cards: Vec<String>,
    pub scenario_cards: Vec<String>,
}

#[derive(Debug)]
pub struct Scenario {
    pub title: String,
    pub set: String,
    pub number: i16,
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
    deluxes.insert("Children of Eorl", "Oaths of the Rohirrim");
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
    // Hall of Beorn has duplicate numbers in the data, so we'll increment ourselves
    let mut standalone_scenario_number = 0;
    let scenarios = hob_scenarios
        .into_iter()
        .filter(|s| !FIRST_AGE_PRODUCTS.contains(&s.product.as_str()))
        .map(|s| {
            // figure out which set we're in
            if s.product != last_deluxe && deluxes.contains_key(s.product.as_str()) {
                last_deluxe = s.product.clone();
            }

            let set = if s.product.parse::<u32>().is_ok()
                || s.product == "The Hunt for the Dreadnaught"
                || s.product == "The Scouring of the Shire"
            {
                standalone_scenario_number += 1;
                "Standalone Scenarios".to_string()
            } else if others.iter().find(|&&o| o == s.product).is_some() {
                s.product.clone()
            } else if s.product != last_deluxe {
                deluxes.get(last_deluxe.as_str()).unwrap().to_string()
            } else {
                last_deluxe.clone()
            };

            let number = if set == "Standalone Scenarios" {
                // Hall of Beorn changed the order of these products, so the scenario counter will
                // be wrong
                if s.title == "Fog on the Barrow-downs" {
                    5
                } else if s.title == "The Ruins of Belegost" {
                    6
                } else {
                    standalone_scenario_number
                }
            } else {
                s.number
            };

            Scenario {
                title: s.title,
                set,
                number,
            }
        })
        .collect();

    Ok(scenarios)
}
