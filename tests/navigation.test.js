import { test } from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { runInNewContext } from 'node:vm'
import { screens } from '../src/screens.js'

test('screenshot navigation wraps and supports arrow, Home and End keys', () => {
  const source = readFileSync(new URL('../src/App.vue', import.meta.url), 'utf8')
  const selected = { value: 0 }
  const script = source.match(/function step\([\s\S]*?(?=<\/script>)/)[0]
  const { step, move } = runInNewContext(script + '\n({ step, move })', { selected, screens })
  step(-1)
  assert.equal(selected.value, 4)
  step(1)
  assert.equal(selected.value, 0)
  let prevented = 0
  const key = key => move({ key, preventDefault() { prevented++ } })
  key('ArrowRight')
  assert.equal(selected.value, 1)
  key('ArrowLeft')
  assert.equal(selected.value, 0)
  key('End')
  assert.equal(selected.value, 4)
  key('Home')
  assert.equal(selected.value, 0)
  key('Tab')
  assert.equal(selected.value, 0)
  assert.equal(prevented, 4)
})
