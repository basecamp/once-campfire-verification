require_relative "comparison_support"

# The reliability profiles' accounting, which runs without Docker.
module Reliability
  extend BenchmarkSupport

  def self.median_or_nil(values)
    values.empty? ? nil : median(values)
  end

  # Medians across rounds for each app, per profile, with each round's failure counts.
  def self.summarize(rows)
    rows.group_by { |row| row.fetch("app") }.to_h do |app, app_rows|
      summary = {}
      overload = app_rows.flat_map { |row| row.fetch("overload", []) }
      unless overload.empty?
        summary["overload"] = overload.group_by { |s| [s.fetch("route"), s.fetch("conc")] }.map do |(route, conc), samples|
          { "route" => route, "conc" => conc, "median_valid_rps" => median(samples.map { |s| s.fetch("rps") }),
            "errors" => samples.sum { |s| s.fetch("errors") }, "invalid_responses" => samples.sum { |s| s.fetch("invalid_responses") },
            "median_p99_ms" => median_or_nil(samples.filter_map { |s| s.dig("latency", "p99_ms") }) }
        end
      end
      %w[cable cable_heavy].each do |profile|
        cable = app_rows.filter_map { |row| row[profile] }
        next if cable.empty?
        summary[profile] = {
          "clients" => cable.first.fetch("clients"),
          "posters" => cable.first.dig("throughput", "posters"),
          "subscribed" => cable.map { |c| c.fetch("ready") },
          "disconnected" => cable.map { |c| c.fetch("disconnected", 0) },
          "delivered_to_all" => cable.map { |c| "#{c.dig('throughput', 'complete')}/#{c.dig('throughput', 'posted')}" },
          "median_delivered_msgs_per_sec" => median(cable.map { |c| c.dig("throughput", "delivered_msgs_per_sec") }),
          "median_all_clients_p99_ms" => median_or_nil(cable.filter_map { |c| c.dig("throughput", "all_clients", "p99_ms") })
        }
      end
      [app, summary]
    end
  end
end
