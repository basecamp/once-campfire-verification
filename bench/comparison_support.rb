require "fileutils"
require "json"
require "open3"
require "optparse"

module BenchmarkSupport
  ROOT = File.expand_path("..", __dir__)
  WORK = File.join(ROOT, "tmp/bench")

  def run(*command, input: nil)
    output, errors, status = Open3.capture3(*command, stdin_data: input, binmode: true)
    raise "#{command.first} failed (#{status.exitstatus}): #{errors}" unless status.success?
    output
  end

  def remove_container(name)
    Open3.capture3("docker", "rm", "-f", name)
  end

  def prepare_storage(seed, destination)
    FileUtils.rm_rf(destination)
    FileUtils.mkdir_p(destination)
    FileUtils.cp_r(File.join(seed, "db"), File.join(destination, "db"))
    FileUtils.cp_r(File.join(seed, "storage"), File.join(destination, "files"))
  end

  def mounts(paths)
    paths.flat_map { |host, target| [ "-v", "#{host}:#{target}" ] }
  end

  def environment(values)
    values.flat_map { |name, value| [ "-e", "#{name}=#{value}" ] }
  end

  def write_json(path, data)
    FileUtils.mkdir_p(File.dirname(path))
    File.write(path, JSON.pretty_generate(data) + "\n")
  end

  def median(values)
    values = values.sort
    middle = values.length / 2
    values.length.odd? ? values[middle] : (values[middle - 1] + values[middle]) / 2.0
  end

  IMAGES = { "rails" => "once-campfire:app", "rust" => "campfire-rust:app", "elixir" => "campfire-elixir:app", "express-bun" => "once-campfire-express:bun" }

  def image_name(app)
    ENV.fetch("#{app.upcase.tr('-', '_')}_IMAGE", IMAGES.fetch(app, "once-campfire-#{app}:app"))
  end

  def source_dir(workspace, app)
    kind = app.delete_suffix("-bun")
    File.join(workspace, kind == "rails" ? "once-campfire" : "once-campfire-#{kind}")
  end

  # Working set matches `docker stats`: usage minus reclaimable inactive file cache.
  # Peak is the kernel's high-water mark and includes page cache (fixture reads).
  def container_memory(container)
    pid = run("docker", "inspect", "-f", "{{.State.Pid}}", container).strip
    path = File.read("/proc/#{pid}/cgroup")[/^0::(\S+)$/, 1] or raise "cgroup v2 is required for memory measurement"
    cgroup_memory(File.join("/sys/fs/cgroup", path))
  end

  # The counters are read separately, so cache reclaim in between can make one sample inconsistent.
  def cgroup_memory(dir, attempts: 3)
    attempts.times do
      stat = File.readlines(File.join(dir, "memory.stat")).to_h { |line| name, value = line.split; [name, Integer(value)] }
      working_set = Integer(File.read(File.join(dir, "memory.current"))) - stat.fetch("inactive_file")
      next if working_set.negative?
      return { "working_set_bytes" => working_set, "anon_bytes" => stat.fetch("anon"),
        "peak_bytes" => Integer(File.read(File.join(dir, "memory.peak"))) }
    end
    raise "inconsistent cgroup memory counters in #{dir}"
  end

  def clock
    Process.clock_gettime(Process::CLOCK_MONOTONIC)
  end
end
