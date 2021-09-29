//! This converts the Con of the Rings achievement as a Tab Separated Value (TSV) into a TOML
//! challenges format that can be loaded with `bin/load_challenges`
use anyhow::anyhow;
use serde::Serialize;

#[derive(Serialize)]
struct Doc {
    code_prefix: String,
    challenge: Vec<Challenge>,
}

#[derive(Serialize)]
struct Challenge {
    name: String,
    description: String,
    scenario: String,
    attributes: Vec<String>,
}

impl Challenge {
    fn new(
        scenario: impl Into<String>,
        attributes: &[String],
        achievement: &str,
    ) -> anyhow::Result<Self> {
        let mut split = achievement.splitn(2, ": ");
        let name = split
            .next()
            .ok_or_else(|| anyhow!(format!("No name found in achievement: {}", achievement)))?
            .to_string();
        let description = split
            .next()
            .ok_or_else(|| {
                anyhow!(format!(
                    "No description found in achievent: {}",
                    achievement
                ))
            })?
            .to_string();

        Ok(Challenge {
            name,
            description,
            scenario: scenario.into(),
            attributes: attributes.into(),
        })
    }
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .delimiter(b'\t')
        .from_path(&args[1])?;

    let mut records_iter = reader.records();
    let scenarios = records_iter
        .next()
        .ok_or_else(|| anyhow!("Could not find scenarios in CSV"))??;
    let achievements1 = records_iter
        .next()
        .ok_or_else(|| anyhow!("Could not find 1st achievements in CSV"))??;
    let achievements2 = records_iter
        .next()
        .ok_or_else(|| anyhow!("Could not find 2nd achievements in CSV"))??;

    if scenarios.len() != achievements1.len() || scenarios.len() != achievements2.len() {
        panic!("Scenarios + Achievements don't have the same number of records");
    }

    let challenges: Vec<Challenge> = scenarios
        .iter()
        .enumerate()
        .map(|(i, scenario)| {
            let attributes = vec![String::from("Con of the Rings 2020")];

            [
                Challenge::new(scenario, &attributes, &achievements1[i]).unwrap(),
                Challenge::new(scenario, &attributes, &achievements2[i]).unwrap(),
            ]
        })
        .flatten()
        .collect();

    let doc = Doc {
        code_prefix: String::from("CON20"),
        challenge: challenges,
    };

    println!("{}", toml::to_string(&doc).unwrap());

    Ok(())
}
