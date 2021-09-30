use crate::models::scenario::Scenario;
use sqlx::postgres::PgPool;

#[derive(Clone)]
pub struct Challenge {
    pub id: i64,
    pub name: String,
    pub code: String,
    pub description: Option<String>,
    pub scenario: Option<Scenario>,
}

impl Challenge {
    /// Fetch all incomplete challenges for an event
    pub async fn find_incompleted_by_event(
        pool: &PgPool,
        event_id: i64,
    ) -> anyhow::Result<Vec<Challenge>> {
        let rows = sqlx::query!(
            r#"
SELECT challenges.id, challenges.name, challenges.code, challenges.description, scenarios.id AS scenario_id, scenarios.title AS scenario_title, scenarios.code AS scenario_code, scenarios.set_id AS scenario_set_id, scenarios.number AS scenario_number
FROM challenges_events, challenges, scenarios
WHERE challenges_events.challenge_id = challenges.id
    AND challenges.scenario_id = scenarios.id
    AND challenges.scenario_id = scenarios.id
    AND challenges_events.event_id = $1
    AND challenges_events.active_date <= CURRENT_TIMESTAMP
    AND challenges.id NOT IN (
        SELECT challenges_events.challenge_id
        FROM challenges_events_users, challenges_events
        WHERE challenges_events_users.challenges_events_id = challenges_events.id
            AND challenges_events.event_id = $1
    )
ORDER BY challenges.code
"#,
event_id
        )
            .fetch_all(pool)
            .await?;

        let challenges = rows
            .into_iter()
            .map(|row| {
                let scenario = Scenario {
                    id: row.scenario_id,
                    title: row.scenario_title.clone(),
                    code: row.scenario_code.clone(),
                    set_id: row.scenario_set_id,
                    number: row.scenario_number,
                };
                Challenge {
                    id: row.id,
                    name: row.name.clone(),
                    code: row.code.clone(),
                    description: Some(row.code.clone()),
                    scenario: Some(scenario),
                }
            })
            .collect();

        Ok(challenges)
    }
}
