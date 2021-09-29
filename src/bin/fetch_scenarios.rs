use erebor_record_keeper::hob_scenario_parser::fetch;
use sqlx::postgres::PgPoolOptions;
use std::collections::HashMap;

fn generate_code(set_id: i64, set_number: i16) -> String {
    format!("{:0>2}{:0>2}", set_id, set_number)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&std::env::var("DATABASE_URL")?)
        .await?;

    let scenarios = fetch().await?;
    let mut sets = HashMap::new();
    for scenario in scenarios {
        let queried_set = sqlx::query!(
            r#"
SELECT id
FROM sets
WHERE name = $1;
"#,
            scenario.set
        )
        .fetch_optional(&pool)
        .await?;

        let set_id = if let Some(set) = queried_set {
            set.id
        } else {
            let set = sqlx::query!(
                r#"
INSERT INTO sets ( name )
VALUES ( $1 )
RETURNING id;
"#,
                scenario.set
            )
            .fetch_one(&pool)
            .await?;

            set.id
        };

        sets.insert(scenario.set.clone(), set_id);

        let existing_scenario = sqlx::query!(
            r#"
SELECT id
FROM scenarios
WHERE title = $1
  AND set_id = $2
  AND number = $3
        "#,
            scenario.title,
            set_id,
            scenario.number
        )
        .fetch_optional(&pool)
        .await?;

        if existing_scenario.is_none() {
            println!("ADDING {:?}", scenario);

            sqlx::query!(
                r#"
INSERT INTO scenarios ( title, set_id, number, code )
VALUES ( $1, $2, $3, $4 )
RETURNING id;
        "#,
                scenario.title,
                set_id,
                scenario.number,
                generate_code(set_id, scenario.number),
            )
            .fetch_one(&pool)
            .await?;
        } else {
            println!("EXISTS {:?}", scenario);
        }
    }

    Ok(())
}
