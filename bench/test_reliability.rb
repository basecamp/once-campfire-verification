require "minitest/autorun"
require "open3"
require "rbconfig"
require "tmpdir"
require_relative "reliability_support"

class ReliabilityTest < Minitest::Test
  COMMAND = File.expand_path("../bin/reliability", __dir__)

  def reliability(*args, directory:)
    Open3.capture3({ "TMPDIR" => directory }, RbConfig.ruby, COMMAND, *args)
  end

  def test_options_are_validated_before_any_runtime_starts
    Dir.mktmpdir do |directory|
      [[%w[--profiles cable,soak], "profiles must be nonempty"],
       [%w[--overload-routes post_message], "overload routes must be read routes"],
       [%w[--apps rails,rails], "apps must be nonempty, unique and supported"],
       [%w[--overload-concurrencies ,], "counts and durations must be positive"],
       [%w[--cable-heavy-posters -1], "cable-heavy-posters must be zero or positive"]].each do |args, message|
        _, errors, status = reliability(*args, directory: directory)
        refute status.success?
        assert_includes errors, message
      end
      output, errors, status = reliability("--help", directory: directory)
      assert status.success?, errors
      assert_includes output, "--profiles"
    end
  end

  def test_reliability_runs_share_the_benchmark_lock
    Dir.mktmpdir do |directory|
      File.open(File.join(directory, "once-campfire-verification-benchmark.lock"), "a") do |lock|
        assert lock.flock(File::LOCK_EX | File::LOCK_NB)
        _, errors, status = reliability("--help", directory: directory)
        refute status.success?
        assert_includes errors, "Another Campfire benchmark is running"
      end
    end
  end

  def test_summary_reports_each_round_and_medians
    overload = ->(rps, errors) { { "route" => "room_show", "conc" => 64, "rps" => rps, "errors" => errors, "invalid_responses" => 0, "latency" => { "p99_ms" => rps / 10.0 } } }
    cable = ->(complete, posted, disconnected) do
      { "clients" => 1000, "ready" => 1000, "disconnected" => disconnected,
        "throughput" => { "posters" => 64, "complete" => complete, "posted" => posted, "delivered_msgs_per_sec" => complete / 10.0, "all_clients" => { "p99_ms" => 50.0 } } }
    end
    rows = [
      { "app" => "rails", "overload" => [overload.(100, 0)], "cable_heavy" => cable.(0, 100, 500) },
      { "app" => "rails", "overload" => [overload.(300, 2)], "cable_heavy" => cable.(10, 100, 499) },
      { "app" => "rails", "overload" => [overload.(200, 1)], "cable_heavy" => cable.(20, 100, 498) }
    ]
    summary = Reliability.summarize(rows).fetch("rails")
    assert_equal [{ "route" => "room_show", "conc" => 64, "median_valid_rps" => 200, "errors" => 3, "invalid_responses" => 0, "median_p99_ms" => 20.0 }],
      summary.fetch("overload")
    assert_equal({ "clients" => 1000, "posters" => 64, "subscribed" => [1000, 1000, 1000], "disconnected" => [500, 499, 498],
      "delivered_to_all" => ["0/100", "10/100", "20/100"], "median_delivered_msgs_per_sec" => 1.0, "median_all_clients_p99_ms" => 50.0 },
      summary.fetch("cable_heavy"))
    refute summary.key?("cable")
  end
end
