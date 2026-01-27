-- Change exploit_jobs.exploit_run_id to SET NULL on delete instead of CASCADE
ALTER TABLE exploit_jobs DROP CONSTRAINT exploit_jobs_exploit_run_id_fkey;
ALTER TABLE exploit_jobs ALTER COLUMN exploit_run_id DROP NOT NULL;
ALTER TABLE exploit_jobs ADD CONSTRAINT exploit_jobs_exploit_run_id_fkey 
    FOREIGN KEY (exploit_run_id) REFERENCES exploit_runs(id) ON DELETE SET NULL;

-- Change flags foreign keys to SET NULL
ALTER TABLE flags DROP CONSTRAINT flags_job_id_fkey;
ALTER TABLE flags ALTER COLUMN job_id DROP NOT NULL;
ALTER TABLE flags ADD CONSTRAINT flags_job_id_fkey 
    FOREIGN KEY (job_id) REFERENCES exploit_jobs(id) ON DELETE SET NULL;
