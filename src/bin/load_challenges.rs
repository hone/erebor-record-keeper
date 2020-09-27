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
    scenario: String,
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
        let scenario = match Scenario::find_by_title(&pool, &challenge.scenario).await? {
            Some(scenario) => scenario,
            None => {
                println!(
                    "Could not find Scenario '{}' from Challenge '{}'",
                    challenge.scenario, challenge.name
                );
                std::process::exit(-1);
            }
        };
        sqlx::query!(
            r#"
INSERT INTO challenges ( name, description, code, scenario_id )
VALUES ( $1, $2, $3, $4 )
"#,
            challenge.name,
            challenge.description,
            code,
            scenario.id
        )
        .execute(&pool)
        .await?;
    }

    Ok(())
}
