{#- The book's code (ticket 2802): the lines of `site/snippets/<file>`
    between `// ANCHOR: <anchor>` and `// ANCHOR_END: <anchor>`, as a
    fenced block in `language` (`rust` unless given; Zola's own `lang`
    is the page's). With `step`, the file is `site/tutorial/<step>/<file>`
    (ticket 2803). With no `anchor`, every line of the file. Lines of other
    anchors inside it are left out, and the lines are shown as they are
    written, so an anchor marks whole items at the file's own indentation.
    A file or an anchor that isn't there fails the build. -#}
{%- if step is defined -%}
{%- set folder = "tutorial/" ~ step ~ "/" -%}
{%- else -%}
{%- set folder = "snippets/" -%}
{%- endif -%}
{%- set source = load_data(path=folder ~ file, format="plain") -%}
{%- if anchor is defined -%}
{%- set wanted = anchor -%}
{%- set_global state = "before" -%}
{%- else -%}
{%- set wanted = "" -%}
{%- set_global state = "in" -%}
{%- endif -%}
{%- set begin = "// ANCHOR: " ~ wanted -%}
{%- set end = "// ANCHOR_END: " ~ wanted -%}
{%- set_global lines = [] -%}
{%- for raw in source | split(pat="\n") -%}
{%- set line = raw | trim_end_matches(pat="\r") -%}
{%- set bare = line | trim -%}
{%- if state == "in" and bare == end -%}
{%- set_global state = "after" -%}
{%- elif state == "in" and bare is not starting_with("// ANCHOR: ") and bare is not starting_with("// ANCHOR_END: ") -%}
{%- set_global lines = lines | concat(with=line) -%}
{%- elif state == "before" and bare == begin -%}
{%- set_global state = "in" -%}
{%- endif -%}
{%- endfor -%}
{%- if state == "before" or (state == "in" and wanted != "") -%}
{{ throw(message="snippet: no `" ~ begin ~ "` … `" ~ end ~ "` in site/" ~ folder ~ file) }}
{%- endif %}
```{{ language | default(value="rust") }}
{{ lines | join(sep="
") }}
```
