-- Add default_counter to exploits
ALTER TABLE exploits ADD COLUMN default_counter INTEGER NOT NULL DEFAULT 999;

-- Exploit Containers (persistent Docker containers)
CREATE TABLE exploit_containers (
    id SERIAL PRIMARY KEY,
    exploit_id INTEGER NOT NULL REFERENCES exploits(id) ON DELETE CASCADE,
    container_id VARCHAR(100) NOT NULL,
    counter INTEGER NOT NULL,
    status VARCHAR(50) NOT NULL DEFAULT 'running',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Exploit Runners (pins exploit_run to container)
CREATE TABLE exploit_runners (
    id SERIAL PRIMARY KEY,
    exploit_container_id INTEGER NOT NULL REFERENCES exploit_containers(id) ON DELETE CASCADE,
    exploit_run_id INTEGER NOT NULL REFERENCES exploit_runs(id) ON DELETE CASCADE,
    team_id INTEGER NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(exploit_run_id)
);

CREATE INDEX idx_exploit_containers_exploit ON exploit_containers(exploit_id);
CREATE INDEX idx_exploit_containers_status ON exploit_containers(status);
CREATE INDEX idx_exploit_runners_container ON exploit_runners(exploit_container_id);
