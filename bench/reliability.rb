# Reliability profiles on the same production images, seed, CPU allocation and response contracts as
# bin/benchmark. Where the benchmark aborts on the first failure, these record what failed:
#
#   overload  the validated read routes past the benchmark's 16 clients, with errors and latency tails
#   cable     Action Cable fan-out to many subscribers of the busy room, with delivery completeness, at the
#             default posting load and again with --cable-heavy-posters closed-loop posters (cable_heavy)
require_relative "comparison_support"
require_relative "http_client"
require_relative "contracts"
require_relative "reliability_support"
require "digest"
require "time"
include BenchmarkSupport

repo = File.expand_path("..", __dir__)
work = File.join(repo, "tmp/bench")
options = { apps: "rails,elixir,go,rust", rounds: 3, port: 25130, workspace: File.dirname(repo),
  seed: File.join(repo, "fixtures/default"), loadgen: ENV.fetch("LOADGEN", File.join(repo, "loadgen/target/release/loadgen")),
  env_file: ENV.fetch("BENCH_ENV_FILE", File.join(repo, "fixtures/default/reference.env")),
  output: File.join(work, "reliability", "#{Time.now.utc.strftime('%Y%m%d-%H%M%S')}-#{Process.pid}"), cpus: "8-11", client_cpus: "12-15",
  profiles: "overload,cable", overload_concurrencies: "64,256", overload_duration: 8, overload_routes: "room_show,messages_page,sidebar,search",
  cable_clients: 1000, cable_tput_secs: 15, cable_posters: 4, cable_heavy_posters: 64 }
OptionParser.new do |parser|
  options.each do |key, default|
    type = default.is_a?(Integer) ? Integer : String
    parser.on("--#{key.to_s.tr('_', '-')} VALUE", type) { |value| options[key] = value }
  end
  parser.on("--help") { puts parser; exit }
end.parse!
apps = options[:apps].split(",")
allowed = %w[rails django laravel express express-bun elixir go rust c cpp]
raise "apps must be nonempty, unique and supported" unless !apps.empty? && apps.uniq == apps && apps.all? { |app| allowed.include?(app) }
profiles = options[:profiles].split(",")
raise "profiles must be nonempty and among overload, cable" unless !profiles.empty? && (profiles - %w[overload cable]).empty?
overload_routes = options[:overload_routes].split(",")
raise "overload routes must be read routes" unless !overload_routes.empty? && (overload_routes - %w[room_show messages_page sidebar search]).empty?
raise "counts and durations must be positive" unless [options[:rounds], options[:overload_duration], options[:cable_clients],
  options[:cable_tput_secs], options[:cable_posters]].all?(&:positive?) &&
  options[:overload_concurrencies].split(",").all? { |value| Integer(value).positive? }
raise "cable-heavy-posters must be zero or positive" if options[:cable_heavy_posters].negative?
raise "output already exists: #{options[:output]}" if File.exist?(options[:output])

runtimes = apps.to_h { |app| [app, app == "express-bun" ? "express" : app] }
env_name = ->(app) { app.upcase.tr("-", "_") }
labels = JSON.parse(File.read(File.join(options[:seed], "labels.json")))
original_seed_sha = Digest::SHA256.file(File.join(options[:seed], "db/production.sqlite3")).hexdigest
room = Integer(labels.fetch("rooms.watercooler"))
base = "http://127.0.0.1:#{options[:port]}"
fixture_env = File.readlines(options[:env_file], chomp: true).reject { |line| line.empty? || line.start_with?("#") }.to_h { |line| line.split("=", 2) }
container = "cf-native-reliability-#{Process.pid}"
lg = ->(*args) { JSON.parse(run("taskset", "-c", options[:client_cpus], options[:loadgen], *args)) }
sql = ->(db, query) do
  readonly = query.match?(/\ASELECT/i)
  output = run("sqlite3", "-cmd", ".timeout 10000", *(readonly ? ["-readonly"] : []), "-json", db, query)
  output.strip.empty? ? [] : JSON.parse(output)
end
metadata = { verification_revision: run("git", "-C", repo, "rev-parse", "HEAD").strip, started_at: Time.now.utc.iso8601, seed_sha256: original_seed_sha,
  server_cpus: options[:cpus], client_cpus: options[:client_cpus], network: "host", rounds: options[:rounds], profiles: profiles,
  loadgen_sha256: Digest::SHA256.file(options[:loadgen]).hexdigest, options: options.except(:output, :workspace, :seed, :loadgen, :env_file),
  images: {}, source_revisions: {} }

# Starts an app on a fresh copy of the seed, exactly as bin/benchmark does.
start_app = ->(app, data) do
  kind = runtimes.fetch(app)
  images = { "rails" => "once-campfire:app", "rust" => "campfire-rust:app", "elixir" => "campfire-elixir:app", "express-bun" => "once-campfire-express:bun" }
  image = ENV.fetch("#{env_name.(app)}_IMAGE", images.fetch(app, "once-campfire-#{app}:app"))
  source = File.join(options[:workspace], kind == "rails" ? "once-campfire" : "once-campfire-#{kind}")
  image_id = run("docker", "image", "inspect", "-f", "{{.Id}}", image).strip
  raise "#{app}: image changed between runs" if metadata[:images].key?(app) && metadata[:images][app] != image_id
  metadata[:images][app] = image_id
  metadata[:source_revisions][app] ||= { head: run("git", "-C", source, "rev-parse", "HEAD").strip,
    dirty: !run("git", "-C", source, "status", "--porcelain", "--untracked-files=no").strip.empty? }
  prepare_storage(options[:seed], data)
  FileUtils.mkdir_p(File.join(data, "logs"))
  db = File.join(data, "db/production.sqlite3")
  sql.call(db, "PRAGMA user_version=1;") if kind == "c"
  sql.call(db, "UPDATE push_subscriptions SET endpoint = 'https://127.0.0.1:9/push/' || id; UPDATE webhooks SET url = 'http://127.0.0.1:9/hook/' || id;")
  config = fixture_env.merge("WEB_CONCURRENCY" => "3", "JOB_CONCURRENCY" => "3", "RAILS_MAX_THREADS" => "5",
    "RAILS_LOG_LEVEL" => "warn", "HTTP_PORT" => options[:port].to_s, "TARGET_PORT" => (options[:port] + 1).to_s)
  config.merge!(JSON.parse(ENV.fetch("#{env_name.(app)}_BENCH_ENV", "{}")))
  config["RELEASE_NODE"] = container if kind == "elixir"
  command = ["docker", "run", "-d", "--name", container, "--network", "host", "--cpuset-cpus", options[:cpus]]
  command.concat environment(config)
  command.concat mounts(File.join(data, "db") => "/rails/storage/db", File.join(data, "files") => "/rails/storage/files", File.join(data, "logs") => "/rails/storage/logs")
  command << image_id
  run(*command)
  deadline = clock + 90
  until BenchmarkHTTPClient.new(base).ready?
    raise "#{app} failed to start" if clock > deadline
    sleep 0.1
  end
  run("docker", "exec", "--user", "root", container, "chmod", "-R", "a+rwX", "/rails/storage/db")
  sleep 3
  db
end

overload = ->(app, data) do
  db = start_app.(app, data)
  cookie = lg.("login", "--base", base, "--email", labels.fetch("emails.david"), "--password", labels.fetch("passwords.all")).fetch("cookie")
  scrape = lg.("scrape", "--base", base, "--cookie", cookie, "--room", room.to_s)
  contracts = BenchmarkContracts.prepare(base, cookie, db, labels, scrape.fetch("css"), File.join(data, "contracts")).fetch(:contracts)
  paths = { "room_show" => "/rooms/#{room}", "messages_page" => "/rooms/#{room}/messages?before=#{labels.fetch('messages.busy_060')}",
    "sidebar" => "/users/me/sidebar", "search" => "/searches?q=coffee" }
  overload_routes.flat_map do |name|
    lg.("http", "--base", base, "--cookie", cookie, "--path", paths.fetch(name), "--validate", contracts.fetch(name), "--conc", "4", "--duration", "2")
    options[:overload_concurrencies].split(",").map do |concurrency|
      value = lg.("http", "--base", base, "--cookie", cookie, "--path", paths.fetch(name), "--validate", contracts.fetch(name),
        "--conc", concurrency, "--duration", options[:overload_duration].to_s)
      puts "#{app}: overload #{name} #{concurrency} clients #{value.fetch('rps')} valid req/s, #{value.fetch('errors')} errors, #{value.fetch('invalid_responses')} invalid"
      STDOUT.flush
      value.slice("conc", "rps", "ok", "errors", "invalid_responses", "invalid_reasons", "statuses", "latency").merge("route" => name)
    end
  end
end

cable = ->(app, data, posters) do
  start_app.(app, data)
  cookie = lg.("login", "--base", base, "--email", labels.fetch("emails.david"), "--password", labels.fetch("passwords.all")).fetch("cookie")
  scrape = lg.("scrape", "--base", base, "--cookie", cookie, "--room", room.to_s)
  lg.("cable", "--base", base, "--cookie", cookie, "--room", room.to_s, "--csrf", scrape.fetch("csrf").to_s,
    "--streams", Array(scrape.fetch("streams")).join(","), "--clients", options[:cable_clients].to_s,
    "--tput-secs", options[:cable_tput_secs].to_s, "--posters", posters.to_s)
end
runs = { "overload" => overload, "cable" => ->(app, data) { cable.(app, data, options[:cable_posters]) },
  "cable_heavy" => ->(app, data) { cable.(app, data, options[:cable_heavy_posters]) } }
# The heavy variant runs on its own fresh container, after the default cable run.
run_order = profiles.flat_map { |profile| profile == "cable" && options[:cable_heavy_posters].positive? ? %w[cable cable_heavy] : [profile] }

rows = []
begin
  options[:rounds].times do |iteration|
    (iteration.even? ? apps : apps.reverse).each do |app|
      row = { "app" => app, "round" => iteration + 1 }
      run_order.each do |profile|
        data = File.join(work, "runtime", Process.pid.to_s, "#{app}-#{profile}-#{iteration + 1}")
        begin
          row[profile] = runs.fetch(profile).(app, data)
        ensure
          logs = File.join(options[:output], "#{app}-#{profile}-#{iteration + 1}.log")
          output, errors, = Open3.capture3("docker", "logs", container)
          FileUtils.mkdir_p(options[:output])
          File.write(logs, output + errors)
          remove_container(container)
          FileUtils.rm_rf(data)
        end
        if profile.start_with?("cable")
          t = row[profile].fetch("throughput")
          puts "#{app} round #{iteration + 1}: #{profile} #{row[profile]['ready']}/#{options[:cable_clients]} subscribed, " \
            "#{row[profile]['disconnected']} disconnected, #{t['complete']}/#{t['posted']} delivered to all, #{t['delivered_msgs_per_sec']} msgs/s"
          STDOUT.flush
        end
      end
      rows << row
      write_json(File.join(options[:output], "#{app}-#{iteration + 1}.json"), row)
    end
  end
  raise "original seed changed" unless Digest::SHA256.file(File.join(options[:seed], "db/production.sqlite3")).hexdigest == original_seed_sha
  metadata[:complete] = true
  summary = Reliability.summarize(rows)
  write_json(File.join(options[:output], "reliability-summary.json"), metadata: metadata, results: summary)
  puts JSON.pretty_generate(summary)
ensure
  remove_container(container)
end
