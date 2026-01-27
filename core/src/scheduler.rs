use crate::{Database, ExploitJob};
use anyhow::Result;

pub struct Scheduler {
    db: Database,
}

impl Scheduler {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    pub fn calculate_priority(challenge_priority: i32, team_priority: i32, sequence: i32, override_priority: Option<i32>) -> i32 {
        override_priority.unwrap_or_else(|| challenge_priority * 10000 + team_priority * 100 + sequence)
    }

    pub async fn generate_round(&self) -> Result<i32> {
        let round = self.db.create_round().await?;
        let challenges = self.db.list_challenges().await?;
        let teams = self.db.list_teams().await?;
        
        let mut jobs = Vec::new();
        
        for challenge in challenges.iter().filter(|c| c.enabled) {
            for team in &teams {
                let runs = self.db.list_exploit_runs(Some(challenge.id), Some(team.id)).await?;
                for run in runs {
                    let priority = Self::calculate_priority(challenge.priority, team.priority, run.sequence, run.priority);
                    jobs.push((run.id, team.id, priority));
                }
            }
        }

        jobs.sort_by(|a, b| b.2.cmp(&a.2)); // Higher priority first

        for (run_id, team_id, priority) in jobs {
            self.db.create_job(round.id, run_id, team_id, priority).await?;
        }

        Ok(round.id)
    }

    pub async fn get_jobs(&self, round_id: i32) -> Result<Vec<ExploitJob>> {
        self.db.list_jobs(round_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_priority_default() {
        assert_eq!(Scheduler::calculate_priority(5, 3, 2, None), 50302);
    }

    #[test]
    fn test_calculate_priority_override() {
        assert_eq!(Scheduler::calculate_priority(5, 3, 2, Some(999)), 999);
    }
}
