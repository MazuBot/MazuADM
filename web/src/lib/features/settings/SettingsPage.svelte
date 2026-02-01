<script>
  import * as api from '$lib/data/api';
  import { formatApiError, pushToast } from '$lib/ui/toastStore.js';

  let { settings, onRefresh } = $props();

  function getSetting(key, fallback) {
    return settings.find((s) => s.key === key)?.value || fallback;
  }

  function parseEnvs(envsJson) {
    if (!envsJson) return [];
    try {
      const obj = JSON.parse(envsJson);
      return Object.entries(obj).map(([key, value]) => ({ key, value }));
    } catch { return []; }
  }

  function serializeEnvs(envsList) {
    if (!envsList || envsList.length === 0) return '';
    const filtered = envsList.filter(e => e.key.trim());
    if (filtered.length === 0) return '';
    const obj = {};
    for (const e of filtered) obj[e.key] = e.value;
    return JSON.stringify(obj);
  }

  let debugEnvs = $state(parseEnvs(getSetting('debug_envs', '')));

  async function updateSettingAndToast(key, value) {
    try {
      await api.updateSetting(key, value);
      await onRefresh();
      pushToast(`Setting ${key} updated.`, 'success');
    } catch (error) {
      pushToast(formatApiError(error, `Failed to update ${key}.`), 'error');
    }
  }

  async function saveDebugEnvs() {
    await updateSettingAndToast('debug_envs', serializeEnvs(debugEnvs));
  }

  $effect(() => {
    debugEnvs = parseEnvs(getSetting('debug_envs', ''));
  });
</script>

<div class="panel">
  <h2>Settings</h2>
  <div class="settings-grid">
    <div class="setting-row">
      <label for="setting_concurrent_limit">concurrent_limit</label>
      <input
        id="setting_concurrent_limit"
        value={getSetting('concurrent_limit', '10')}
        onchange={(e) => updateSettingAndToast('concurrent_limit', e.target.value)}
      />
      <span class="setting-desc">Max concurrent job executions</span>
    </div>
    <div class="setting-row">
      <label for="setting_concurrent_create_limit">concurrent_create_limit</label>
      <input
        id="setting_concurrent_create_limit"
        value={getSetting('concurrent_create_limit', '1')}
        onchange={(e) => updateSettingAndToast('concurrent_create_limit', e.target.value)}
      />
      <span class="setting-desc">Max concurrent container creations</span>
    </div>
    <div class="setting-row">
      <label for="setting_worker_timeout">worker_timeout</label>
      <input
        id="setting_worker_timeout"
        value={getSetting('worker_timeout', '60')}
        onchange={(e) => updateSettingAndToast('worker_timeout', e.target.value)}
      />
      <span class="setting-desc">Default job timeout in seconds</span>
    </div>
    <div class="setting-row">
      <label for="setting_max_flags_per_job">max_flags_per_job</label>
      <input
        id="setting_max_flags_per_job"
        value={getSetting('max_flags_per_job', '50')}
        onchange={(e) => updateSettingAndToast('max_flags_per_job', e.target.value)}
      />
      <span class="setting-desc">Max flags extracted per job</span>
    </div>
    <div class="setting-row">
      <label for="setting_container_output_limit">container_output_limit</label>
      <input
        id="setting_container_output_limit"
        value={getSetting('container_output_limit', '256000')}
        onchange={(e) => updateSettingAndToast('container_output_limit', e.target.value)}
      />
      <span class="setting-desc">Max output bytes saved per job</span>
    </div>
    <div class="setting-row">
      <label for="setting_past_flag_rounds">past_flag_rounds</label>
      <input
        id="setting_past_flag_rounds"
        value={getSetting('past_flag_rounds', '5')}
        onchange={(e) => updateSettingAndToast('past_flag_rounds', e.target.value)}
      />
      <span class="setting-desc">Rounds before current allowed for flag submission</span>
    </div>
    <div class="setting-row">
      <label for="setting_skip_on_flag">skip_on_flag</label>
      <select id="setting_skip_on_flag" onchange={(e) => updateSettingAndToast('skip_on_flag', e.target.value)}>
        <option value="false" selected={getSetting('skip_on_flag', 'false') !== 'true'}>No</option>
        <option value="true" selected={getSetting('skip_on_flag', 'false') === 'true'}>Yes</option>
      </select>
      <span class="setting-desc">Skip remaining jobs for team after flag</span>
    </div>
    <div class="setting-row">
      <label for="setting_sequential_per_target">sequential_per_target</label>
      <select
        id="setting_sequential_per_target"
        onchange={(e) => updateSettingAndToast('sequential_per_target', e.target.value)}
      >
        <option value="false" selected={getSetting('sequential_per_target', 'false') !== 'true'}>No</option>
        <option value="true" selected={getSetting('sequential_per_target', 'false') === 'true'}>Yes</option>
      </select>
      <span class="setting-desc">Run jobs sequentially per target</span>
    </div>
    <div class="setting-row">
      <label for="setting_ip_headers">ip_headers</label>
      <input
        id="setting_ip_headers"
        value={getSetting('ip_headers', '')}
        placeholder="X-Forwarded-For,X-Real-IP"
        onchange={(e) => updateSettingAndToast('ip_headers', e.target.value)}
      />
      <span class="setting-desc">Comma-separated headers for client IP detection</span>
    </div>
    <div class="setting-row">
      <label for="setting_default_ignore_connection_info">default_ignore_connection_info</label>
      <select
        id="setting_default_ignore_connection_info"
        onchange={(e) => updateSettingAndToast('default_ignore_connection_info', e.target.value)}
      >
        <option value="false" selected={getSetting('default_ignore_connection_info', 'false') !== 'true'}>No</option>
        <option value="true" selected={getSetting('default_ignore_connection_info', 'false') === 'true'}>Yes</option>
      </select>
      <span class="setting-desc">Default value for exploit ignore_connection_info</span>
    </div>
  </div>
  <div class="env-section">
    <span class="env-label">debug_envs <span class="setting-desc">- Global environment variables for all jobs</span></span>
    <div class="env-list">
      {#each debugEnvs as env, i}
        <div class="env-row">
          <input type="text" bind:value={env.key} placeholder="KEY" />
          <input type="text" bind:value={env.value} placeholder="value" />
          <button type="button" class="small danger" onclick={() => debugEnvs = debugEnvs.filter((_, idx) => idx !== i)}>×</button>
        </div>
      {/each}
    </div>
    <div class="env-actions">
      <button type="button" class="small" onclick={() => debugEnvs = [...debugEnvs, { key: '', value: '' }]}>+ Add Env</button>
      <button type="button" class="small" onclick={saveDebugEnvs}>Save</button>
    </div>
  </div>
</div>

<style>
  .setting-desc {
    color: #888;
    font-size: 0.85em;
  }
  .env-section { margin: 1.5rem 0 0 0; }
  .env-label { display: block; color: #aaa; font-size: 0.9rem; margin-bottom: 0.5rem; }
  .env-list { max-height: 160px; overflow-y: scroll; }
  .env-row { display: flex; gap: 0.5rem; margin-bottom: 0.5rem; }
  .env-row input { flex: 1; padding: 0.4rem; background: #1a1a2e; border: 1px solid #444; color: #eee; border-radius: 4px; }
  .env-row input:first-child { max-width: 150px; }
  .env-actions { display: flex; gap: 0.5rem; }
</style>
