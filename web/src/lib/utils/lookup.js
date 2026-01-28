export function getTeamName(teams, teamId) {
  const t = teams?.find((team) => team.id === teamId)
  return t ? `${t.id} (${t.team_name})` : (teamId ?? '-')
}

export function getTeamId(teams, teamId) {
  const t = teams?.find((team) => team.id === teamId)
  return t ? t.team_id : (teamId ?? '-')
}

export function getTeamDisplay(teams, teamId) {
  const t = teams?.find((team) => team.id === teamId)
  return t ? `${t.team_id} (${t.team_name})` : (teamId ?? '-')
}

export function getChallengeName(challenges, challengeId) {
  const c = challenges?.find((challenge) => challenge.id === challengeId)
  return c ? c.name : (challengeId ?? '-')
}

export function getExploitName(exploits, exploitId) {
  const e = exploits?.find((exploit) => exploit.id === exploitId)
  return e ? e.name : (exploitId ?? '-')
}

export function getExploitRunName(exploitRuns, exploits, runId) {
  const run = exploitRuns?.find((r) => r.id === runId)
  return run ? getExploitName(exploits, run.exploit_id) : (runId ?? '-')
}
