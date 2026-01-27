-- Challenges
CREATE TABLE challenges (
    id SERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT true,
    default_port INTEGER,
    priority INTEGER NOT NULL DEFAULT 0,
    flag_regex VARCHAR(512),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Teams
CREATE TABLE teams (
    id SERIAL PRIMARY KEY,
    team_id VARCHAR(100) NOT NULL UNIQUE,
    team_name VARCHAR(255) NOT NULL,
    default_ip VARCHAR(255),
    priority INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Challenge-Team Relations (columns in Trello view)
CREATE TABLE challenge_team_relations (
    id SERIAL PRIMARY KEY,
    challenge_id INTEGER NOT NULL REFERENCES challenges(id) ON DELETE CASCADE,
    team_id INTEGER NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    addr VARCHAR(255),
    port INTEGER,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(challenge_id, team_id)
);

-- Exploits
CREATE TABLE exploits (
    id SERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    challenge_id INTEGER NOT NULL REFERENCES challenges(id) ON DELETE CASCADE,
    enabled BOOLEAN NOT NULL DEFAULT true,
    priority INTEGER NOT NULL DEFAULT 0,
    max_per_container INTEGER NOT NULL DEFAULT 1,
    docker_image VARCHAR(512) NOT NULL,
    entrypoint VARCHAR(512),
    timeout_secs INTEGER NOT NULL DEFAULT 30,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Exploit Runs (cards in Trello view)
CREATE TABLE exploit_runs (
    id SERIAL PRIMARY KEY,
    exploit_id INTEGER NOT NULL REFERENCES exploits(id) ON DELETE CASCADE,
    challenge_id INTEGER NOT NULL REFERENCES challenges(id) ON DELETE CASCADE,
    team_id INTEGER NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    priority INTEGER,
    sequence INTEGER NOT NULL DEFAULT 0,
    enabled BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(exploit_id, challenge_id, team_id)
);

-- Rounds
CREATE TABLE rounds (
    id SERIAL PRIMARY KEY,
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    finished_at TIMESTAMPTZ,
    status VARCHAR(50) NOT NULL DEFAULT 'pending'
);

-- Exploit Jobs
CREATE TABLE exploit_jobs (
    id SERIAL PRIMARY KEY,
    round_id INTEGER NOT NULL REFERENCES rounds(id) ON DELETE CASCADE,
    exploit_run_id INTEGER NOT NULL REFERENCES exploit_runs(id) ON DELETE CASCADE,
    team_id INTEGER NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    priority INTEGER NOT NULL,
    status VARCHAR(50) NOT NULL DEFAULT 'pending',
    container_id VARCHAR(100),
    stdout TEXT,
    stderr TEXT,
    duration_ms INTEGER,
    started_at TIMESTAMPTZ,
    finished_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Flags
CREATE TABLE flags (
    id SERIAL PRIMARY KEY,
    job_id INTEGER NOT NULL REFERENCES exploit_jobs(id) ON DELETE CASCADE,
    round_id INTEGER NOT NULL REFERENCES rounds(id) ON DELETE CASCADE,
    challenge_id INTEGER NOT NULL REFERENCES challenges(id) ON DELETE CASCADE,
    team_id INTEGER NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    flag_value VARCHAR(512) NOT NULL,
    status VARCHAR(50) NOT NULL DEFAULT 'captured',
    submitted_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_exploit_jobs_round ON exploit_jobs(round_id);
CREATE INDEX idx_exploit_jobs_status ON exploit_jobs(status);
CREATE INDEX idx_flags_round ON flags(round_id);
CREATE INDEX idx_flags_status ON flags(status);
