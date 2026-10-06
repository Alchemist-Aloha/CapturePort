<script setup>
import { ref } from 'vue'

const installer = 'https://raw.githubusercontent.com/Alchemist-Aloha/CapturePort/main/scripts/install.sh'
const command = `curl -fsSL ${installer} | sh`
const status = ref('')
const copying = ref(false)
async function copy() {
  if (copying.value) return
  copying.value = true
  status.value = ''
  try {
    await navigator.clipboard.writeText(command)
    status.value = 'Copied to clipboard.'
  } catch {
    status.value = 'Copy unavailable. Select the command and copy it manually.'
  } finally {
    copying.value = false
  }
}
</script>

<template>
  <div class="install-command">
    <div class="command-panel" :class="{ 'command-copied': status === 'Copied to clipboard.' }">
      <div class="command-header">
        <p class="install-label">Install with one command</p>
        <button type="button" @click="copy" :disabled="copying" aria-label="Copy install command">
          <svg viewBox="0 0 24 24" aria-hidden="true"><path v-if="status === 'Copied to clipboard.'" class="copy-check" pathLength="1" d="m5 12 4 4L19 6" /><template v-else><rect x="8" y="8" width="12" height="12" rx="2" /><path d="M16 8V4a2 2 0 0 0-2-2H4a2 2 0 0 0-2 2v10a2 2 0 0 0 2 2h4" /></template></svg>
          {{ copying ? 'Copying…' : status === 'Copied to clipboard.' ? 'Copied' : 'Copy' }}
        </button>
      </div>
      <code tabindex="0" aria-label="Linux install command">{{ command }}</code>
    </div>
    <div class="command-footer">
      <p>Linux x86_64 <span aria-hidden="true">·</span> No sudo needed</p>
      <a :href="installer" target="_blank" rel="noopener">Review installer <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M7 17 17 7M7 7h10v10" /></svg></a>
    </div>
    <p class="copy-status" :class="{ 'copy-error': status && status !== 'Copied to clipboard.' }" role="status">{{ status }}</p>
  </div>
</template>

<style scoped>
.install-command{max-width:920px;margin:32px auto 0;text-align:left}
.command-panel{padding:12px 20px 20px;background:#24221e;border:1px solid var(--line);border-radius:12px;transition:border-color .2s ease}
.command-panel.command-copied{border-color:var(--accent)}
.copy-check{stroke-dasharray:1;animation:confirm-copy .24s var(--settle) both}
@keyframes confirm-copy{from{stroke-dashoffset:1}to{stroke-dashoffset:0}}
.command-header{display:flex;align-items:center;justify-content:space-between;gap:12px;margin-bottom:10px}
.install-label{font-size:15px;font-weight:600;line-height:1.4;text-wrap:balance}
code{display:block;font-family:ui-monospace,monospace;font-size:13px;line-height:1.8;white-space:pre-wrap;overflow-wrap:anywhere;user-select:all}
button{display:inline-flex;align-items:center;justify-content:center;gap:8px;min-width:90px;min-height:44px;padding:10px;border:0;border-radius:8px;background:transparent;color:var(--accent);font-size:13px;flex-shrink:0;transition:background .2s ease}
button:hover:not(:disabled){background:#2d2923}
button:active:not(:disabled){background:#433a2e}
button:disabled{cursor:wait;color:var(--muted)}
button svg{width:16px;height:16px}
.command-footer{display:flex;align-items:center;justify-content:space-between;gap:8px 20px;flex-wrap:wrap;margin-top:8px;font-size:12px;line-height:1.6;color:var(--muted)}
.command-footer p{padding-block:8px}
.command-footer p span{margin-inline:4px}
.command-footer a{display:inline-flex;align-items:center;gap:4px;min-height:36px;text-decoration:underline}
.command-footer svg{width:14px;height:14px}
.copy-status{position:absolute;width:1px;height:1px;padding:0;margin:-1px;overflow:hidden;clip-path:inset(50%);white-space:nowrap}
.copy-status.copy-error{position:static;width:auto;height:auto;margin:4px 0 0;overflow:visible;clip-path:none;white-space:normal;font-size:13px;line-height:1.6;color:var(--accent)}
@media(max-width:600px){.command-panel{padding:10px 16px 18px}.install-label{font-size:14px}code{font-size:12px}.command-footer{gap:0 12px}}
</style>
