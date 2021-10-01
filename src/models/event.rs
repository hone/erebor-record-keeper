use crate::models::{challenge::Challenge, scenario::Scenario};
use sqlx::postgres::PgPool;

pub struct Event {
    pub id: i64,
    pub name: String,
}

impl Event {
    /// Find the active Event
    pub async fn find_by_active(pool: &PgPool, active: bool) -> anyhow::Result<Option<Event>> {
        let mut rows = sqlx::query_as!(
            Event,
            r#"
SELECT id, name
FROM events
WHERE active = $1
"#,
            active
        )
        .fetch_all(pool)
        .await?;

        Ok(rows.pop())
    }

    /// Find events by archive status
    pub async fn find_by_archive(pool: &PgPool, archive: bool) -> anyhow::Result<Vec<Event>> {
        Ok(sqlx::query_as!(
            Event,
            r#"
SELECT id, name
FROM events
WHERE archive = $1
"#,
            archive
        )
        .fetch_all(pool)
        .await?)
    }

    /// Create new event
    pub async fn create(pool: &PgPool, name: &str) -> anyhow::Result<u64> {
        Ok(
            sqlx::query!(r#"INSERT INTO events ( name ) VALUES ( $1 )"#, name)
                .execute(pool)
                .await?
                .rows_affected(),
        )
    }

    /// Completed Challenges
    pub async fn find_completed_challenges(&self, pool: &PgPool) -> anyhow::Result<Vec<Challenge>> {
        let rows = sqlx::query!(
            r#"
SELECT challenges.id, challenges.name, challenges.code, challenges.description, scenarios.id AS scenario_id, scenarios.title AS scenario_title, scenarios.code AS scenario_code, scenarios.set_id AS scenario_set_id, scenarios.number AS scenario_number
FROM challenges_events, challenges, scenarios
WHERE challenges_events.event_id = $1
    AND challenges_events.challenge_id = challenges.id
    AND challenges.scenario_id = scenarios.id
    AND challenges_events.active_date <= CURRENT_TIMESTAMP
    AND challenges.id IN (
        SELECT challenges_events.challenge_id
        FROM challenges_events_users, challenges_events
        WHERE challenges_events_users.challenges_events_id = challenges_events.id
            AND challenges_events.event_id = $1
    )
ORDER by challenges.code
"#,
            self.id
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

    /// Fetch all challenges
    pub async fn find_all_challenges(&self, pool: &PgPool) -> anyhow::Result<Vec<Challenge>> {
        let rows = sqlx::query!(
            r#"
SELECT challenges.id, challenges.name, challenges.code, challenges.description, scenarios.id AS scenario_id, scenarios.title AS scenario_title, scenarios.code AS scenario_code, scenarios.set_id AS scenario_set_id, scenarios.number AS scenario_number
FROM challenges_events, challenges, scenarios
WHERE challenges_events.challenge_id = challenges.id
    AND challenges.scenario_id = scenarios.id
    AND challenges.scenario_id = scenarios.id
    AND challenges_events.event_id = $1
ORDER BY challenges.code
"#,
            self.id
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

    /// Fetch all active challenges
    pub async fn find_all_active_challenges(
        &self,
        pool: &PgPool,
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
ORDER BY challenges.code
"#,
            self.id
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

    /// Fetch all incomplete active_challenges for an event
    pub async fn find_incomplete_active_challenges(
        &self,
        pool: &PgPool,
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
            self.id
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
