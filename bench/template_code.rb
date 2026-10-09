# Extract server-side template code while preserving original physical lines.
# Pass the output to cloc using the backend language; never count generated code.
module TemplateCode
  SYNTAXES = %w[erb eex eta blade jinja askama go].freeze
  BLADE_DIRECTIVES = %w[
    extends section endsection show yield include includeIf includeWhen includeUnless
    includeFirst each if elseif else endif unless endunless isset endisset empty endempty
    foreach endforeach forelse endforelse for endfor while endwhile switch endswitch
    case default break continue php endphp auth endauth guest endguest can endcan
    cannot endcannot canany endcanany error enderror csrf method checked selected disabled
    readonly required class style once endonce push endpush prepend endprepend stack
    pushOnce endPushOnce prependOnce endPrependOnce inject props aware component endcomponent
    slot endslot hasSection sectionMissing endhasSection session endsession use
  ].freeze

  def self.extract(source, syntax:)
    raise ArgumentError, "unsupported template syntax: #{syntax}" unless SYNTAXES.include?(syntax)

    scanner = Scanner.new(source)
    case syntax
    when "erb", "eex", "eta" then scanner.percent
    when "blade" then scanner.blade
    else scanner.curly(syntax)
    end
    scanner.output
  end

  class Scanner
    attr_reader :output

    def initialize(source)
      @source = source
      @output = source.gsub(/[^\n]/, " ")
    end

    def mark(first, last)
      @output[first...last] = @source[first...last]
    end

    def required_index(token, first)
      @source.index(token, first) || raise(ArgumentError, "unterminated template region at offset #{first}")
    end

    # Expressions may contain quoted delimiters, parentheses and code comments.
    def close(first, token)
      quote = nil
      comment = nil
      depth = 0
      i = first
      while i < @source.length
        char = @source[i]
        if comment == :line
          comment = nil if char == "\n"
        elsif comment == :block
          if @source[i, 2] == "*/"
            comment = nil
            i += 2
            next
          end
        elsif quote
          if char == "\\" && quote != "`"
            i += 2
            next
          end
          quote = nil if char == quote
        elsif depth.zero? && @source[i, token.length] == token
          return i
        elsif @source[i, 2] == "/*"
          comment = :block
          i += 2
          next
        elsif @source[i, 2] == "//"
          comment = :line
          i += 2
          next
        elsif ["'", '"', "`"].include?(char)
          quote = char
        elsif char == "(" || char == "[" || char == "{"
          depth += 1
        elsif char == ")" || char == "]" || char == "}"
          depth -= 1
        end
        i += 1
      end
      raise ArgumentError, "unterminated code region at offset #{first}"
    end

    def percent
      pos = 0
      while (first = @source.index("<%", pos))
        if @source[first, 5] == "<%!--"
          pos = required_index("--%>", first + 5) + 4
          next
        end
        last = required_index("%>", first + 2)
        marker = @source[first + 2]
        unless ["#", "%"].include?(marker)
          start = first + 2
          start += 1 if ["=", "~", "-", "_"].include?(marker)
          finish = last
          finish -= 1 if ["-", "_"].include?(@source[last - 1])
          mark(start, finish)
        end
        pos = last + 2
      end
    end

    def curly(syntax)
      pos = 0
      pattern = syntax == "go" ? /\{\{/ : /\{\{|\{%|\{#/
      while (tag = pattern.match(@source, pos))
        first = tag.end(0)
        if tag[0] == "{#"
          last = required_index("#}", first)
          if syntax == "askama"
            depth = 1
            cursor = first
            while (comment = /\{#|#\}/.match(@source, cursor))
              depth += comment[0] == "{#" ? 1 : -1
              cursor = comment.end(0)
              if depth.zero?
                last = comment.begin(0)
                break
              end
            end
            raise ArgumentError, "unterminated nested comment" unless depth.zero?
          end
          pos = last + 2
          next
        end
        token = tag[0] == "{%" ? "%}" : "}}"
        last = close(first, token)
        text = @source[first...last]
        if tag[0] == "{%" && text.strip.match?(/\A[-+]?\s*raw\s*[-+]?\z/)
          ending = /\{%[-+]?\s*endraw\s*[-+]?%\}/.match(@source, last + 2)
          raise ArgumentError, "unterminated raw block" unless ending
          pos = ending.end(0)
          next
        end
        unless syntax == "go" && text.lstrip.sub(/\A-\s*/, "").start_with?("/*")
          start = first + (["-", "+"].include?(@source[first]) ? 1 : 0)
          finish = last - (["-", "+"].include?(@source[last - 1]) ? 1 : 0)
          mark(start, finish)
        end
        pos = last + 2
      end
    end

    def blade
      pos = 0
      pattern = /\{\{--|@verbatim\b|@@[A-Za-z_]\w*|@\{\{|@([A-Za-z_]\w*)|\{!!|\{\{|<\?(?:php|=)/
      while (tag = pattern.match(@source, pos))
        start = tag.begin(0)
        first = tag.end(0)
        case tag[0]
        when "{{--"
          pos = required_index("--}}", first) + 4
        when "@verbatim"
          pos = required_index("@endverbatim", first) + "@endverbatim".length
        when "@{{"
          pos = close(first, "}}") + 2
        when /\A@@/
          pos = first
        when "{{", "{!!", "<?php", "<?="
          token = { "{{" => "}}", "{!!" => "!!}", "<?php" => "?>", "<?=" => "?>" }.fetch(tag[0])
          last = close(first, token)
          mark(first, last)
          pos = last + token.length
        else
          name = tag[1]
          unless BLADE_DIRECTIVES.include?(name)
            pos = first
            next
          end
          args = /\s*\(/.match(@source, first)
          if args && args.begin(0) == first
            opening = args.end(0) - 1
            last = close(opening + 1, ")")
            if name == "php"
              mark(opening + 1, last)
            else
              mark(start + 1, last + 1)
            end
            pos = last + 1
          elsif name == "php"
            last = required_index("@endphp", first)
            mark(first, last)
            pos = last + "@endphp".length
          else
            mark(start + 1, first) unless name == "endphp"
            pos = first
          end
        end
      end
    end
  end
end

if $PROGRAM_NAME == __FILE__
  syntax, file = ARGV
  abort "Usage: ruby bench/template_code.rb #{TemplateCode::SYNTAXES.join('|')} TEMPLATE" unless syntax && file && ARGV.length == 2
  STDOUT.write(TemplateCode.extract(File.read(file), syntax: syntax))
end
