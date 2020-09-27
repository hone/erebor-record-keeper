-- Add migration script here
CREATE TABLE IF NOT EXISTS users
(
	id         BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
	discord_id BIGINT NOT NULL UNIQUE,
	name       VARCHAR(255),
	created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
	updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

ALTER TABLE events_scenarios
ADD COLUMN checkout_user_id BIGINT,
DROP COLUMN checkout_user,
ADD CONSTRAINT events_scenarios_checkout_user_id_fkey
FOREIGN KEY (checkout_user_id)
REFERENCES users(id);

CREATE TABLE IF NOT EXISTS challenges_events_users
(
	id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
	challenges_events_id BIGINT NOT NULL,
	user_id BIGINT NOT NULL,
	created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
	updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
	FOREIGN KEY(challenges_events_id) REFERENCES challenges_events(id),
	FOREIGN KEY(user_id) REFERENCES users(id)
);

CREATE UNIQUE INDEX ON challenges_events_users (challenges_events_id, user_id);
