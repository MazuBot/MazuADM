ALTER TABLE exploit_jobs
    ALTER COLUMN create_reason TYPE VARCHAR(20)
    USING SUBSTRING(create_reason FROM 1 FOR 20);
