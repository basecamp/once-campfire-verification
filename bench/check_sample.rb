require "json"
value = JSON.parse(STDIN.read)
if ARGV.first == "cable"
  raise "Incomplete Cable subscriptions" unless value.fetch("ready") == value.fetch("clients") && value.fetch("failed").zero?
  { "latency" => "messages", "throughput" => "posted" }.each do |phase, count|
    sample = value.fetch(phase)
    posted = sample.fetch(count)
    raise "Unsuccessful Cable messages: #{sample}" unless posted.positive? && sample.fetch("post_attempts") == posted && sample.fetch("post_errors").zero? && sample.fetch("complete") == posted
  end
else
  samples = value["profile"] == "mixed-read-write-v1" ? [value, value.fetch("writer")] : [value]
  samples.each do |sample|
    raise "Invalid benchmark sample: #{sample}" unless sample.fetch("errors").zero? && sample.fetch("invalid_responses").zero? && sample.fetch("validation") == "route-contract-v1" && sample.fetch("ok") > 0 && sample.fetch("statuses") == { "200" => sample.fetch("ok") }
  end
end
puts JSON.generate(value)
