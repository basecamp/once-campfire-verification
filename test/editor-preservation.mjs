import assert from 'node:assert/strict'
import fs from 'node:fs'
import vm from 'node:vm'

function element(picker = false) {
  const attributes = new Map()
  return {
    querySelector(selector) { return selector.includes('autocomplete') && picker ? {} : null },
    hasAttribute(name) { return attributes.has(name) },
    setAttribute(name, value) { attributes.set(name, value) },
    toggleAttribute(name, value) { value ? attributes.set(name, '') : attributes.delete(name) },
  }
}
const sources = process.argv.slice(2)
if (!sources.length) throw new Error("Pass one or more rooms_list_controller.js source paths")
for (const filename of sources) {
  const listeners = new Map()
  const context = vm.createContext({ Controller: class {}, cable: { async subscribeTo() {return {unsubscribe(){}}} }, ignoringBriefDisconnects: (_e,fn)=>fn() })
  vm.runInContext(fs.readFileSync(filename,'utf8').replace(/^import .*\n/gm,'').replace('export default class','globalThis.ControllerUnderTest = class'),context)
  const controller = new context.ControllerUnderTest()
  let current = element(true), incoming = element()
  controller.element = {id:'user_sidebar',addEventListener(name,fn){listeners.set(name,fn)},removeEventListener(name,fn){assert.equal(listeners.get(name),fn);listeners.delete(name)},querySelector(){return current}}
  await controller.connect()
  function render(target = controller.element, perform = () => {}) {
    const event = {target,detail:{newFrame:{querySelector(){return incoming}},render:perform}}
    listeners.get('turbo:before-frame-render')(event)
    return event
  }
  const preserved = render(controller.element,()=>{
    assert.equal(current.hasAttribute('data-turbo-permanent'),true)
    assert.equal(incoming.hasAttribute('data-turbo-permanent'),true)
    return 'rendered'
  })
  assert.equal(preserved.detail.render(),'rendered')
  assert.equal(current.hasAttribute('data-turbo-permanent'),false)
  assert.equal(incoming.hasAttribute('data-turbo-permanent'),false)
  const failed = render(controller.element,()=>{throw new Error('render failed')})
  assert.throws(()=>failed.detail.render(),/render failed/)
  assert.equal(current.hasAttribute('data-turbo-permanent'),false)
  assert.equal(incoming.hasAttribute('data-turbo-permanent'),false)
  current.setAttribute('data-turbo-permanent','')
  render().detail.render()
  assert.equal(current.hasAttribute('data-turbo-permanent'),true)
  current.toggleAttribute('data-turbo-permanent',false)
  render({id:'direct_rooms_control'}).detail.render()
  assert.equal(current.hasAttribute('data-turbo-permanent'),false,'Own picker navigation must not be intercepted')
  listeners.get('turbo:click')({target:{closest(){return {dataset:{turboFrame:'user_sidebar'}}}}})
  render({id:'direct_rooms_control'}).detail.render()
  render().detail.render()
  assert.equal(current.hasAttribute('data-turbo-permanent'),false,'Explicit parent navigation must dismiss picker')
  const subsequent=render()
  assert.equal(current.hasAttribute('data-turbo-permanent'),true,'Dismiss intent must not leak into later refreshes')
  subsequent.detail.render()
  current=element(false)
  render().detail.render()
  assert.equal(current.hasAttribute('data-turbo-permanent'),false,'Inactive direct lists must refresh')
  current=element(true);incoming=null
  render().detail.render()
  assert.equal(current.hasAttribute('data-turbo-permanent'),false,'Missing incoming control must not be preserved')
  controller.disconnect()
  assert.equal(listeners.size,0,'Disconnect removes event handlers')
  console.log(`${filename}: editor preservation/navigation/cleanup regression passed`)
}
