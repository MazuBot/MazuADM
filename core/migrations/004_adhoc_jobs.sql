-- Allow ad-hoc jobs without a round
ALTER TABLE exploit_jobs ALTER COLUMN round_id DROP NOT NULL;
ALTER TABLE exploit_jobs DROP CONSTRAINT exploit_jobs_round_id_fkey;
ALTER TABLE exploit_jobs ADD CONSTRAINT exploit_jobs_round_id_fkey 
    FOREIGN KEY (round_id) REFERENCES rounds(id) ON DELETE CASCADE;

ALTER TABLE flags ALTER COLUMN round_id DROP NOT NULL;
ALTER TABLE flags DROP CONSTRAINT flags_round_id_fkey;
ALTER TABLE flags ADD CONSTRAINT flags_round_id_fkey 
    FOREIGN KEY (round_id) REFERENCES rounds(id) ON DELETE CASCADE;

ALTER TABLE flags ALTER COLUMN job_id DROP NOT NULL;
ALTER TABLE flags DROP CONSTRAINT flags_job_id_fkey;
ALTER TABLE flags ADD CONSTRAINT flags_job_id_fkey 
    FOREIGN KEY (job_id) REFERENCES exploit_jobs(id) ON DELETE SET NULL;
