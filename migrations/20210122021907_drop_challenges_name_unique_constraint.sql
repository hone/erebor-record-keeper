-- Add migration script here
ALTER TABLE challenges DROP CONSTRAINT challenges_name_key;
CREATE INDEX challenges_name_key ON challenges (name);
