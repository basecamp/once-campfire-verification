import assert from "node:assert/strict"
import fs from "node:fs"
import vm from "node:vm"

function deferred() {
  let resolve, reject
  const promise = new Promise((yes, no) => { resolve = yes; reject = no })
  return { promise, resolve, reject }
}

async function fixture(source) {
  let callbacks
  const pending = deferred()
  let reloads = 0
  const context = vm.createContext({
    Controller: class {},
    cable: { async subscribeTo(_channel, listeners) { callbacks = listeners; return { unsubscribe() {} } } },
    ignoringBriefDisconnects: (_element, callback) => callback(),
  })
  const code = source.replace(/^import .*\n/gm, "").replace("export default class", "globalThis.RoomsListController = class")
  vm.runInContext(code, context)
  const controller = new context.RoomsListController()
  controller.element = { addEventListener() {}, removeEventListener() {}, loaded: pending.promise, isConnected: true, reload() { reloads++ } }
  await controller.connect()
  return { controller, get callbacks() { return callbacks }, pending, reloads: () => reloads }
}

async function check(source) {
  const first = await fixture(source)
  const connected = first.callbacks.connected()
  assert.equal(first.reloads(), 0, "A connection must not abort the initial frame response")
  first.pending.resolve()
  await connected
  assert.equal(first.reloads(), 1, "Initial connection reloads once after loading finishes")
  await first.callbacks.connected()
  assert.equal(first.reloads(), 1, "Duplicate connected notification must not reload")
  first.callbacks.disconnected()
  await first.callbacks.connected()
  assert.equal(first.reloads(), 2, "A real reconnect still refreshes the sidebar")

  const removed = await fixture(source)
  const removedConnection = removed.callbacks.connected()
  removed.controller.element.isConnected = false
  removed.pending.resolve()
  await removedConnection
  assert.equal(removed.reloads(), 0, "A removed frame must not reload")

  const disconnected = await fixture(source)
  const staleConnection = disconnected.callbacks.connected()
  disconnected.callbacks.disconnected()
  disconnected.pending.resolve()
  await staleConnection
  assert.equal(disconnected.reloads(), 0, "A disconnected channel must not reload")

  const reconnected = await fixture(source)
  const oldConnection = reconnected.callbacks.connected()
  reconnected.callbacks.disconnected()
  const currentConnection = reconnected.callbacks.connected()
  reconnected.pending.resolve()
  await Promise.all([oldConnection, currentConnection])
  assert.equal(reconnected.reloads(), 1, "Only the latest connection may refresh after an interrupted load")

  const detached = await fixture(source)
  const detachedConnection = detached.callbacks.connected()
  detached.controller.element.isConnected = false
  detached.controller.disconnect()
  detached.pending.resolve()
  await detachedConnection
  assert.equal(detached.reloads(), 0, "An unsubscribed controller must not reload")
  detached.controller.element.isConnected = true
  await detached.controller.connect()
  await detached.callbacks.connected()
  assert.equal(detached.reloads(), 1, "Reattaching a controller must refresh through its new subscription")

  const failed = await fixture(source)
  const failedConnection = failed.callbacks.connected()
  const failure = new Error("Actual frame load failure")
  failed.pending.reject(failure)
  await assert.rejects(failedConnection, error => error === failure, "Genuine load errors must remain visible")
  assert.equal(failed.reloads(), 0)
}

const sources = process.argv.slice(2)
if (!sources.length) throw new Error("Pass one or more rooms_list_controller.js source paths")
for (const filename of sources) {
  await check(fs.readFileSync(filename, "utf8"))
  console.log(`${filename}: sidebar loading/reconnect regression passed`)
}
