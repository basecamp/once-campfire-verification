require "minitest/autorun"
require_relative "template_code"

class TemplateCodeTest < Minitest::Test
  def extract(source, syntax)
    result = TemplateCode.extract(source, syntax: syntax)
    assert_equal source.count("\n"), result.count("\n")
    result
  end

  def lines(source, syntax)
    extract(source, syntax).lines.reject { |line| line.strip.empty? }
  end

  def test_plain_markup_and_browser_code_are_excluded
    source = "<h1>Title</h1>\n<script>const x = 1;</script>\n<style>body { color: red; }</style>\n"
    TemplateCode::SYNTAXES.each { |syntax| assert_empty lines(source, syntax) }
  end

  def test_percent_expressions_and_multiline_code_keep_physical_lines
    source = "<p><%= user.name %><%= user.title %></p>\n<% if user.active? %>\n<p>Plain</p>\n<%\n  render(\n    user\n  )\n%>\n<% end %>\n"
    result = extract(source, "erb")
    assert_equal 6, result.lines.count { |line| !line.strip.empty? }
    assert_includes result.lines[0], "user.name"
    assert_includes result.lines[0], "user.title"
    refute_includes result, "<p>"
  end

  def test_percent_comments_and_escaped_tags
    source = "<%# hidden\n<%= not_code %>\n%>\n<%%= browser %>\n<%= real %>\n"
    assert_equal 1, lines(source, "erb").length
  end

  def test_eex_and_eta_output_markers
    assert_equal 2, lines("<%= @name %>\n<% if @ready do %>\nplain\n", "eex").length
    assert_equal 1, lines("<%~ include('item') %><%= item.name %>\n", "eta").length
  end

  def test_eex_comments_can_contain_template_markers
    source = "<%!-- comment <%= hidden %> --%>\n<%= @real %>\n"
    assert_equal 1, lines(source, "eex").length
  end

  def test_curly_expressions_comments_and_literal_blocks
    source = "<h1>{{ user.name }}</h1>\n{% if active %}\nplain\n{% endif %}\n{# {{ hidden }} #}\n{% raw %}{{ literal }}{% endraw %}\n"
    %w[jinja askama].each { |syntax| assert_equal 3, lines(source, syntax).length }
  end

  def test_quoted_delimiters_and_balanced_structures
    assert_equal 1, lines(%({{ lookup({"key": "}}"}) }}\n), "jinja").length
    assert_equal 1, lines(%({{ "}}" }}\n), "go").length
    assert_equal 1, lines(%Q|@if(call(')', [nested(1)]))ok@endif\n|, "blade").length
  end

  def test_go_comments_and_trim_markers
    source = "{{/* comment }} still a comment */}}\n{{- if .Active -}}\n<p>plain</p>\n{{ .Name }}\n{{ end }}\n"
    assert_equal 3, lines(source, "go").length
  end

  def test_askama_nested_comments
    assert_equal 1, lines("{# outer {# nested #} {{ hidden }} #}\n{{ real }}\n", "askama").length
  end

  def test_blade_directives_echoes_and_php_blocks
    source = "@if($active)\n<p>{{ $name }} {!! html($body) !!}</p>\n<p>plain</p>\n@endif\n@php\n$x = 1;\n$y = 2;\n@endphp\n@php($z = 3)\n"
    assert_equal 6, lines(source, "blade").length
  end

  def test_blade_comments_escaped_syntax_and_browser_events
    source = "{{-- @if($hidden) {{ hidden }} --}}\n@verbatim\n{{ browser }} @if(x)\n@endverbatim\n@{{ browser }}\n@@if(browser)\n<div data-action=\"event@window->go event@document->go\">plain</div>\n{{ $real }}\n"
    assert_equal 1, lines(source, "blade").length
  end

  def test_unterminated_code_fails
    assert_raises(ArgumentError) { TemplateCode.extract("{{ missing", syntax: "jinja") }
    assert_raises(ArgumentError) { TemplateCode.extract("@if(unclosed", syntax: "blade") }
  end
end
