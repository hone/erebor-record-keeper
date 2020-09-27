-- Add migration script here
CREATE TABLE IF NOT EXISTS challenges
(
	id          BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
	name        VARCHAR(255) NOT NULL UNIQUE,
	code        VARCHAR(255) NOT NULL UNIQUE,
	description TEXT,
	scenario_id BIGINT,
	attributes  VARCHAR(255) [],
	created_at  TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
	updated_at  TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
	FOREIGN KEY(scenario_id) REFERENCES scenarios(id)
);

CREATE TABLE IF NOT EXISTS challenges_events
(
	id 		BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
	event_id	BIGINT NOT NULL,
	challenge_id 	BIGINT NOT NULL,
	created_at 	TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
	updated_at 	TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
	FOREIGN KEY(event_id) REFERENCES events(id),
	FOREIGN KEY(challenge_id) REFERENCES challenges(id)
);

CREATE UNIQUE INDEX ON challenges_events (event_id, challenge_id);
