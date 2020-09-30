use erebor_record_keeper::models::scenario::Scenario;
use serde::Deserialize;
use sqlx::postgres::PgPoolOptions;

#[derive(Deserialize)]
struct Challenges {
    code_prefix: String,
    challenge: Vec<Challenge>,
}

#[derive(Deserialize)]
struct Challenge {
    name: String,
    description: String,
    scenario: Option<String>,
    #[serde(default)]
    attributes: Vec<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let contents = std::fs::read_to_string(&args[1])?;

    dotenv::dotenv().ok();

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&std::env::var("DATABASE_URL")?)
        .await?;

    let doc: Challenges = toml::from_str(&contents).unwrap();

    for (i, challenge) in doc.challenge.iter().enumerate() {
        let code = format!("{}{:0>2}", doc.code_prefix, i + 1);
        let scenario_id = if let Some(scenario) = &challenge.scenario {
            match Scenario::find_by_title(&pool, &scenario).await? {
                Some(scenario) => Some(scenario.id),
                None => {
                    println!(
                        "Could not find Scenario '{}' from Challenge '{}'",
                        scenario, challenge.name
                    );
                    std::process::exit(-1);
                }
            }
        } else {
            None
        };
        sqlx::query!(
            r#"
INSERT INTO challenges ( name, description, code, scenario_id, attributes )
VALUES ( $1, $2, $3, $4, $5 )
"#,
            &challenge.name,
            &challenge.description,
            code,
            scenario_id,
            &challenge.attributes
        )
        .execute(&pool)
        .await?;
    }

    Ok(())
}
