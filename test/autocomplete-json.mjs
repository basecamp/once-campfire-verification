import assert from "node:assert/strict"
import fs from "node:fs"
import vm from "node:vm"
import { createServer } from "node:http"

assert.ok(process.argv.length > 2, "Pass one or more BaseAutocompleteHandler source paths")
for (const filename of process.argv.slice(2)) {
  const records = [{ name: "Joined browser user", value: 42 }]
  let receivedAccept, receivedQuery
  const server = createServer((request, response) => {
    receivedAccept = request.headers.accept
    receivedQuery = new URL(request.url, "http://localhost").searchParams.get("query")
    const json = receivedAccept === "application/json"
    response.writeHead(200, { "Content-Type": json ? "application/json" : "text/html" })
    response.end(json ? JSON.stringify(records) : "<p>Joined browser user</p>")
  })
  await new Promise(resolve => server.listen(0, "127.0.0.1", resolve))
  try {
    const context = vm.createContext({
      fetch,
      Collection: class {
        constructor(values) { this.values = values }
        matchingQuery() { return { toArray: () => this.values } }
      },
      SuggestionController: class {},
      generateUUID: () => "fixture",
    })
    const source = fs.readFileSync(filename, "utf8")
      .replace(/^import .*\n/gm, "")
      .replace("export default class BaseAutocompleteHandler", "globalThis.BaseAutocompleteHandler = class BaseAutocompleteHandler")
    vm.runInContext(source, context)
    const handler = new context.BaseAutocompleteHandler({ id: "users", dataset: {} }, `http://127.0.0.1:${server.address().port}/autocompletable/users`)
    await new Promise(resolve => handler.loadAutocompletables("Joined", resolve))
    assert.equal(receivedAccept, "application/json")
    assert.equal(receivedQuery, "Joined")
    assert.deepEqual(handler.autocompletablesMatchingQuery("Joined"), records)
    console.log(`${filename}: real HTTP JSON negotiation and autocomplete records passed`)
  } finally {
    server.closeAllConnections()
    await new Promise(resolve => server.close(resolve))
  }
}
