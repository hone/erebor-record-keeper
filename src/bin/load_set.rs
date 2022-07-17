use serde::Deserialize;
use sqlx::postgres::PgPoolOptions;

#[derive(Deserialize)]
struct Set {
    name: String,
    scenarios: Vec<Scenario>,
}

#[derive(Deserialize)]
struct Scenario {
    title: String,
    number: i16,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let contents = std::fs::read_to_string(&args[1])?;
    let set: Set = toml::from_str(&contents)?;

    dotenv::dotenv().ok();

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&std::env::var("DATABASE_URL")?)
        .await?;

    let set_id = match sqlx::query!(
        r#"
INSERT INTO sets ( name )
VALUES ( $1 )
RETURNING id
"#,
        set.name
    )
    .fetch_one(&pool)
    .await
    {
        Ok(record) => record.id,
        Err(err) => {
            if let sqlx::Error::Database(_) = err {
                // if the record already exists, fetch the set instead of erroring out
                sqlx::query!(
                    r#"
SELECT id
FROM sets
WHERE name = $1
"#,
                    set.name
                )
                .fetch_one(&pool)
                .await?
                .id
            } else {
                Err(err)?
            }
        }
    };

    for scenario in set.scenarios {
        let code = format!("{:0>2}{:0>2}", set_id, scenario.number);
        if let Err(err) = sqlx::query!(
            r#"
INSERT INTO scenarios ( title, code, set_id, number )
VALUES ( $1, $2, $3, $4 )
"#,
            scenario.title,
            code,
            set_id,
            scenario.number
        )
        .execute(&pool)
        .await
        {
            // don't error out if the scenario already exists
            if let sqlx::Error::Database(_) = err {
            } else {
                Err(err)?
            }
        }
    }

    Ok(())
}
