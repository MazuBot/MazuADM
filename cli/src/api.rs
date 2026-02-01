use anyhow::{anyhow, Result};
use reqwest::Client;
use tracing::debug;
use crate::models::*;

fn parse_response<T: serde::de::DeserializeOwned>(text: &str) -> Result<T> {
    serde_json::from_str(text).map_err(|_| anyhow!("{}", text))
}

pub struct ApiClient {
    client: Client,
    base_url: String,
}

impl ApiClient {
    pub fn new(base_url: &str) -> Self {
        Self { client: Client::new(), base_url: base_url.trim_end_matches('/').to_string() }
    }

    fn url(&self, path: &str) -> String { format!("{}{}", self.base_url, path) }

    pub async fn get_version(&self) -> Result<VersionInfo> { self.get("/api/version").await }

    async fn get<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T> {
        let resp = self.client.get(self.url(path)).send().await?;
        if !resp.status().is_success() {
            return Err(anyhow!("{}: {}", resp.status(), resp.text().await?));
        }
        let text = resp.text().await?;
        debug!("GET {} response: {}", path, text);
        parse_response(&text)
    }

    async fn post<T: serde::de::DeserializeOwned, B: serde::Serialize>(&self, path: &str, body: &B) -> Result<T> {
        let resp = self.client.post(self.url(path)).json(body).send().await?;
        if !resp.status().is_success() {
            return Err(anyhow!("{}: {}", resp.status(), resp.text().await?));
        }
        let text = resp.text().await?;
        debug!("POST {} response: {}", path, text);
        parse_response(&text)
    }

    async fn post_empty<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T> {
        let resp = self.client.post(self.url(path)).send().await?;
        if !resp.status().is_success() {
            return Err(anyhow!("{}: {}", resp.status(), resp.text().await?));
        }
        let text = resp.text().await?;
        debug!("POST {} response: {}", path, text);
        parse_response(&text)
    }

    async fn put<T: serde::de::DeserializeOwned, B: serde::Serialize>(&self, path: &str, body: &B) -> Result<T> {
        let resp = self.client.put(self.url(path)).json(body).send().await?;
        if !resp.status().is_success() {
            return Err(anyhow!("{}: {}", resp.status(), resp.text().await?));
        }
        let text = resp.text().await?;
        debug!("PUT {} response: {}", path, text);
        parse_response(&text)
    }

    async fn delete(&self, path: &str) -> Result<()> {
        let resp = self.client.delete(self.url(path)).send().await?;
        if !resp.status().is_success() {
            return Err(anyhow!("{}: {}", resp.status(), resp.text().await?));
        }
        Ok(())
    }

    async fn patch<T: serde::de::DeserializeOwned, B: serde::Serialize>(&self, path: &str, body: &B) -> Result<T> {
        let resp = self.client.patch(self.url(path)).json(body).send().await?;
        if !resp.status().is_success() {
            return Err(anyhow!("{}: {}", resp.status(), resp.text().await?));
        }
        let text = resp.text().await?;
        debug!("PATCH {} response: {}", path, text);
        parse_response(&text)
    }

    // Challenges
    pub async fn list_challenges(&self) -> Result<Vec<Challenge>> { self.get("/api/challenges").await }
    pub async fn create_challenge(&self, c: CreateChallenge) -> Result<Challenge> { self.post("/api/challenges", &c).await }
    pub async fn update_challenge(&self, id: i32, c: CreateChallenge) -> Result<Challenge> { self.put(&format!("/api/challenges/{}", id), &c).await }
    pub async fn delete_challenge(&self, id: i32) -> Result<()> { self.delete(&format!("/api/challenges/{}", id)).await }
    pub async fn set_challenge_enabled(&self, id: i32, enabled: bool) -> Result<()> {
        let resp = self.client.put(self.url(&format!("/api/challenges/{}/enabled/{}", id, enabled))).send().await?;
        if !resp.status().is_success() { return Err(anyhow!("{}", resp.text().await?)); }
        Ok(())
    }

    // Teams
    pub async fn list_teams(&self) -> Result<Vec<Team>> { self.get("/api/teams").await }
    pub async fn create_team(&self, t: CreateTeam) -> Result<Team> { self.post("/api/teams", &t).await }
    pub async fn update_team(&self, id: i32, t: CreateTeam) -> Result<Team> { self.put(&format!("/api/teams/{}", id), &t).await }
    pub async fn delete_team(&self, id: i32) -> Result<()> { self.delete(&format!("/api/teams/{}", id)).await }

    // Exploits
    pub async fn list_exploits(&self, challenge_id: Option<i32>) -> Result<Vec<Exploit>> {
        let path = match challenge_id {
            Some(id) => format!("/api/exploits?challenge_id={}", id),
            None => "/api/exploits".to_string(),
        };
        self.get(&path).await
    }
    pub async fn create_exploit(&self, e: CreateExploit) -> Result<Exploit> { self.post("/api/exploits", &e).await }
    pub async fn update_exploit(&self, id: i32, e: UpdateExploit) -> Result<Exploit> { self.put(&format!("/api/exploits/{}", id), &e).await }
    pub async fn delete_exploit(&self, id: i32) -> Result<()> { self.delete(&format!("/api/exploits/{}", id)).await }

    // Exploit Runs
    pub async fn list_exploit_runs(&self, challenge_id: Option<i32>, team_id: Option<i32>) -> Result<Vec<ExploitRun>> {
        let mut params = vec![];
        if let Some(id) = challenge_id { params.push(format!("challenge_id={}", id)); }
        if let Some(id) = team_id { params.push(format!("team_id={}", id)); }
        let path = if params.is_empty() { "/api/exploit-runs".to_string() } else { format!("/api/exploit-runs?{}", params.join("&")) };
        self.get(&path).await
    }
    pub async fn create_exploit_run(&self, r: CreateExploitRun) -> Result<ExploitRun> { self.post("/api/exploit-runs", &r).await }
    pub async fn update_exploit_run(&self, id: i32, u: UpdateExploitRun) -> Result<ExploitRun> { self.put(&format!("/api/exploit-runs/{}", id), &u).await }
    pub async fn reorder_exploit_runs(&self, items: Vec<ReorderExploitRunItem>) -> Result<()> { let _: String = self.post("/api/exploit-runs/reorder", &items).await?; Ok(()) }
    pub async fn delete_exploit_run(&self, id: i32) -> Result<()> { self.delete(&format!("/api/exploit-runs/{}", id)).await }

    // Rounds
    pub async fn list_rounds(&self) -> Result<Vec<Round>> { self.get("/api/rounds").await }
    pub async fn get_current_round(&self) -> Result<Option<Round>> { self.get("/api/rounds/current").await }
    pub async fn create_round(&self) -> Result<i32> { self.post_empty("/api/rounds").await }
    pub async fn run_round(&self, id: i32) -> Result<()> { let _: String = self.post_empty(&format!("/api/rounds/{}/run", id)).await?; Ok(()) }
    pub async fn rerun_round(&self, id: i32) -> Result<()> { let _: String = self.post_empty(&format!("/api/rounds/{}/rerun", id)).await?; Ok(()) }
    pub async fn rerun_unflagged_round(&self, id: i32) -> Result<()> { let _: String = self.post_empty(&format!("/api/rounds/{}/rerun-unflagged", id)).await?; Ok(()) }

    // Jobs
    pub async fn list_jobs(&self, round_id: i32) -> Result<Vec<ExploitJob>> { self.get(&format!("/api/jobs?round_id={}", round_id)).await }
    pub async fn get_job(&self, id: i32) -> Result<ExploitJob> { self.get(&format!("/api/jobs/{}", id)).await }
    pub async fn enqueue_single_job(&self, req: EnqueueSingleJobRequest) -> Result<ExploitJob> { self.post("/api/jobs/enqueue", &req).await }
    pub async fn enqueue_existing_job(&self, id: i32) -> Result<ExploitJob> { self.post_empty(&format!("/api/jobs/{}/enqueue", id)).await }
    pub async fn stop_job(&self, id: i32) -> Result<ExploitJob> { self.post_empty(&format!("/api/jobs/{}/stop", id)).await }
    pub async fn reorder_jobs(&self, items: Vec<ReorderJobItem>) -> Result<()> { let _: String = self.post("/api/jobs/reorder", &items).await?; Ok(()) }

    // Flags
    pub async fn list_flags(&self, round_id: Option<i32>) -> Result<Vec<Flag>> {
        let path = match round_id {
            Some(id) => format!("/api/flags?round_id={}", id),
            None => "/api/flags".to_string(),
        };
        self.get(&path).await
    }

    pub async fn submit_flag(&self, req: SubmitFlagRequest) -> Result<Flag> {
        self.post("/api/flags", &req).await
    }

    pub async fn update_flags(&self, items: Vec<UpdateFlagRequest>, force: bool) -> Result<Vec<bool>> {
        let path = if force { "/api/flags?force=true" } else { "/api/flags" };
        self.patch(path, &items).await
    }

    pub async fn init_flag_cache(&self) -> Result<()> { let _: String = self.post_empty("/api/flags/cache/init").await?; Ok(()) }

    // Settings
    pub async fn list_settings(&self) -> Result<Vec<Setting>> { self.get("/api/settings").await }
    pub async fn update_setting(&self, s: UpdateSetting) -> Result<()> { let _: String = self.post("/api/settings", &s).await?; Ok(()) }

    // Containers
    pub async fn list_containers(&self) -> Result<Vec<ContainerInfo>> { self.get("/api/containers").await }
    pub async fn get_container_runners(&self, id: &str) -> Result<Vec<ExploitJob>> { self.get(&format!("/api/containers/{}/runners", id)).await }
    pub async fn delete_container(&self, id: &str) -> Result<()> { self.delete(&format!("/api/containers/{}", id)).await }
    pub async fn restart_container(&self, id: &str) -> Result<()> { let _: String = self.post_empty(&format!("/api/containers/{}/restart", id)).await?; Ok(()) }
    pub async fn restart_all_containers(&self) -> Result<ContainerBulkOpResult> { self.post_empty("/api/containers/restart-all").await }
    pub async fn remove_all_containers(&self) -> Result<ContainerBulkOpResult> { self.post_empty("/api/containers/remove-all").await }

    // WebSocket connections
    pub async fn list_ws_connections(&self) -> Result<Vec<WsConnection>> { self.get("/api/ws-connections").await }

    // Relations
    pub async fn list_relations(&self, challenge_id: i32) -> Result<Vec<ChallengeTeamRelation>> { self.get(&format!("/api/relations/{}", challenge_id)).await }
    pub async fn get_relation(&self, challenge_id: i32, team_id: i32) -> Result<Option<ChallengeTeamRelation>> { self.get(&format!("/api/relations/{}/{}", challenge_id, team_id)).await }
    pub async fn update_connection_info(&self, challenge_id: i32, team_id: i32, u: UpdateConnectionInfo) -> Result<ChallengeTeamRelation> { self.put(&format!("/api/relations/{}/{}", challenge_id, team_id), &u).await }
}
