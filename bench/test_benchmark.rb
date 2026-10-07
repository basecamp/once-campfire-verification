require "minitest/autorun"
require "open3"
require "rbconfig"
require "tmpdir"

class BenchmarkExclusionTest < Minitest::Test
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
