use serde::Deserialize;
use sqlx::postgres::PgPoolOptions;

const CODE_PREFIX: &str = "ROTK2021-MC";

#[derive(Deserialize)]
struct Kang {
    id: i64,
    name: String,
    description: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    let args: Vec<String> = std::env::args().collect();
    let mut reader = csv::Reader::from_path(&args[1])?;
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&std::env::var("DATABASE_URL")?)
        .await?;
    let base_attributes: Vec<String> = vec![
        "Marvel Champions",
        "Council of 100 Kangs",
        "LeacgueCG Con 2021: The Return of the Kang",
    ]
    .into_iter()
    .map(|s| String::from(s))
    .collect();

    for kang in reader.deserialize::<Kang>().map(|row| row.unwrap()) {
        for mode in [String::from("Standard"), String::from("Expert")].iter() {
            let code = format!(
                "{}{}{:0>2}",
                CODE_PREFIX,
                &mode.chars().nth(0).unwrap().to_uppercase(),
                &kang.id
            );
            let mut attributes = base_attributes.clone();
            attributes.push(mode.clone());
            sqlx::query!(
                r#"
INSERT INTO challenges ( name, description, code, attributes )
VALUES ( $1, $2, $3, $4 )
"#,
                format!("{} ({})", &kang.name, mode),
                &kang.description,
                &code,
                &attributes,
            )
            .execute(&pool)
            .await?;
        }
    }

    Ok(())
}
