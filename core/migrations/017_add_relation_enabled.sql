-- Add enabled field to challenge_team_relations
ALTER TABLE challenge_team_relations ADD COLUMN enabled BOOLEAN NOT NULL DEFAULT true;
