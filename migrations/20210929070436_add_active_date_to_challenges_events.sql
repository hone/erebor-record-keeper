-- Add migration script here
ALTER TABLE challenges_events ADD COLUMN active_date TIMESTAMP;
