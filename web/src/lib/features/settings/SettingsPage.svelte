<script>
  import * as api from '$lib/data/api';
  import { formatApiError, pushToast } from '$lib/ui/toastStore.js';

  let { settings, onRefresh } = $props();

  function getSetting(key, fallback) {
    return settings.find((s) => s.key === key)?.value || fallback;
  }

  async function updateSettingAndToast(key, value) {
    try {
      await api.updateSetting(key, value);
      await onRefresh();
      pushToast(`Setting ${key} updated to ${value}.`, 'success');
    } catch (error) {
      pushToast(formatApiError(error, `Failed to update ${key}.`), 'error');
    }
  }
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
    </div>
    <div class="setting-row">
      <label for="setting_concurrent_create_limit">concurrent_create_limit</label>
      <input
        id="setting_concurrent_create_limit"
        value={getSetting('concurrent_create_limit', '1')}
        onchange={(e) => updateSettingAndToast('concurrent_create_limit', e.target.value)}
      />
    </div>
    <div class="setting-row">
      <label for="setting_worker_timeout">worker_timeout</label>
      <input
        id="setting_worker_timeout"
        value={getSetting('worker_timeout', '60')}
        onchange={(e) => updateSettingAndToast('worker_timeout', e.target.value)}
      />
    </div>
    <div class="setting-row">
      <label for="setting_max_flags_per_job">max_flags_per_job</label>
      <input
        id="setting_max_flags_per_job"
        value={getSetting('max_flags_per_job', '50')}
        onchange={(e) => updateSettingAndToast('max_flags_per_job', e.target.value)}
      />
    </div>
    <div class="setting-row">
      <label for="setting_skip_on_flag">skip_on_flag</label>
      <select id="setting_skip_on_flag" onchange={(e) => updateSettingAndToast('skip_on_flag', e.target.value)}>
        <option value="false" selected={getSetting('skip_on_flag', 'false') !== 'true'}>No</option>
        <option value="true" selected={getSetting('skip_on_flag', 'false') === 'true'}>Yes</option>
      </select>
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
    </div>
  </div>
  <p class="hint">skip_on_flag: Skip remaining exploits for a chal/team once a flag is found in this round.</p>
  <p class="hint">sequential_per_target: Run exploits sequentially per chal/team (don't run multiple exploits for same target at once).</p>
</div>
