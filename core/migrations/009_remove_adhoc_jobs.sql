-- Remove ad-hoc jobs/flags and require round_id
DELETE FROM flags WHERE round_id IS NULL;
DELETE FROM exploit_jobs WHERE round_id IS NULL;

ALTER TABLE exploit_jobs ALTER COLUMN round_id SET NOT NULL;

ALTER TABLE flags ALTER COLUMN round_id SET NOT NULL;
