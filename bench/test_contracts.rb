require "minitest/autorun"
require_relative "contracts"

class AvatarContractTest < Minitest::Test
  def test_decodes_a_real_image_and_rejects_truncated_and_counterfeit_image_bytes
    image = "R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7".unpack1("m0")
    BenchmarkContracts.validate_avatar(image)
    assert_raises(RuntimeError) { BenchmarkContracts.validate_avatar(image.byteslice(0, 20)) }
    assert_raises(RuntimeError) { BenchmarkContracts.validate_avatar("<!doctype html><html>error page</html>") }
  end
  def test_decodes_reference_webp_with_ancillary_exif_and_rejects_corrupt_pixels
    image = File.binread(File.expand_path("../test/fixtures/avatar.webp", __dir__))
    BenchmarkContracts.validate_avatar(image)
    assert_raises(RuntimeError) { BenchmarkContracts.validate_avatar(image.byteslice(0, 100)) }
    corrupt = image.dup
    index = corrupt.index("VP8 ") || raise("fixture missing VP8 image chunk")
    corrupt[index + 8, 100] = "\0" * 100
    assert_raises(RuntimeError) { BenchmarkContracts.validate_avatar(corrupt) }
  end
end
