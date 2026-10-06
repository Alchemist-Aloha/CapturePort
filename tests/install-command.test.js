import { test } from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { runInNewContext } from 'node:vm'
import { ref } from 'vue'

test('install command copies the official installer and handles clipboard failure', async () => {
  const component = readFileSync(new URL('../src/InstallCommand.vue', import.meta.url), 'utf8')
  const script = component.match(/<script setup>([\s\S]*?)<\/script>/)[1].replace("import { ref } from 'vue'", '')
  let copied
  const clipboard = { writeText: async text => { copied = text } }
  const { command, copy, status, copying } = runInNewContext(script + '\n({ command, copy, status, copying })', { ref, navigator: { clipboard } })
  assert.equal(command, 'curl -fsSL https://raw.githubusercontent.com/Alchemist-Aloha/CapturePort/main/scripts/install.sh | sh')
  await copy()
  assert.equal(copied, command)
  assert.equal(status.value, 'Copied to clipboard.')
  assert.equal(copying.value, false)
  let finish
  let calls = 0
  clipboard.writeText = () => { calls++; return new Promise(resolve => { finish = resolve }) }
  const inProgress = copy()
  assert.equal(copying.value, true)
  assert.equal(status.value, '')
  await copy()
  assert.equal(calls, 1)
  finish()
  await inProgress
  assert.equal(copying.value, false)
  clipboard.writeText = async () => { throw new Error('Permission denied') }
  await copy()
  assert.match(status.value, /copy it manually/)
  assert.equal(copying.value, false)
  delete clipboard.writeText
  await copy()
  assert.match(status.value, /copy it manually/)
})
