require "minitest/autorun"
require "open3"
require "rbconfig"
require "tmpdir"
require_relative "comparison_support"

class BenchmarkExclusionTest < Minitest::Test
  def test_mixed_profile_rate_is_bounded_before_any_runtime_starts
    command = File.expand_path("../bin/benchmark", __dir__)
    Dir.mktmpdir do |directory|
      [-1, 101].each do |rate|
        _, errors, status = Open3.capture3({ "TMPDIR" => directory }, RbConfig.ruby, command, "--mixed-write-rate", rate.to_s)
        refute status.success?
        assert_includes errors, "mixed write rate must be between 0 and 100"
      end
      output, errors, status = Open3.capture3({ "TMPDIR" => directory }, RbConfig.ruby, command, "--help")
      assert status.success?, errors
      assert_includes output, "--mixed-write-rate"
    end
  end

  def test_another_run_cannot_start_while_the_benchmark_lock_is_held
    Dir.mktmpdir do |directory|
      command = File.expand_path("../bin/benchmark", __dir__)
      File.open(File.join(directory, "once-campfire-verification-benchmark.lock"), "a") do |lock|
        assert lock.flock(File::LOCK_EX | File::LOCK_NB)
        _, errors, status = Open3.capture3({ "TMPDIR" => directory }, RbConfig.ruby, command, "--help")
        refute status.success?
        assert_includes errors, "Another Campfire benchmark is running"
      end
      output, errors, status = Open3.capture3({ "TMPDIR" => directory }, RbConfig.ruby, command, "--help")
      assert status.success?, errors
      assert_includes output, "--apps"
    end
  end

  def test_cgroup_memory_excludes_reclaimable_file_cache
    Dir.mktmpdir do |directory|
      File.write(File.join(directory, "memory.current"), "1000\n")
      File.write(File.join(directory, "memory.peak"), "1500\n")
      File.write(File.join(directory, "memory.stat"), "anon 600\nfile 400\ninactive_file 300\n")
      memory = Object.new.extend(BenchmarkSupport).cgroup_memory(directory)
      assert_equal({ "working_set_bytes" => 700, "anon_bytes" => 600, "peak_bytes" => 1500 }, memory)
    end
  end

  def test_cgroup_memory_rejects_persistently_inconsistent_counters
    Dir.mktmpdir do |directory|
      File.write(File.join(directory, "memory.current"), "100\n")
      File.write(File.join(directory, "memory.peak"), "1500\n")
      File.write(File.join(directory, "memory.stat"), "anon 600\ninactive_file 300\n")
      error = assert_raises(RuntimeError) { Object.new.extend(BenchmarkSupport).cgroup_memory(directory) }
      assert_includes error.message, "inconsistent cgroup memory counters"
    end
  end

  def test_build_images_rejects_unknown_apps_before_building
    Dir.mktmpdir do |directory|
      _, errors, status = Open3.capture3({ "TMPDIR" => directory }, RbConfig.ruby, File.expand_path("../bin/build-images", __dir__), "--apps", "cobol")
      refute status.success?
      assert_includes errors, "apps must be nonempty, unique and supported"
    end
  end
end
