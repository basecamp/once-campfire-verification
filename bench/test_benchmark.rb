require "minitest/autorun"
require "open3"
require "rbconfig"
require "tmpdir"

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
end
