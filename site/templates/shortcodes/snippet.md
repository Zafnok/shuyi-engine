{#- The book's code (ticket 2802): the lines of `site/snippets/<file>`
    between `// ANCHOR: <anchor>` and `// ANCHOR_END: <anchor>`, as a
    fenced block in `language` (`rust` unless given; Zola's own `lang`
    is the page's). Lines of other anchors
    inside it are left out, and the lines are shown as they are written,
    so an anchor marks whole items at the file's own indentation. A file
    or an anchor that isn't there fails the build. -#}
{%- set source = load_data(path="snippets/" ~ file, format="plain") -%}
{%- set begin = "// ANCHOR: " ~ anchor -%}
{%- set end = "// ANCHOR_END: " ~ anchor -%}
{%- set_global state = "before" -%}
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
{%- if state != "after" -%}
{{ throw(message="snippet: no `" ~ begin ~ "` … `" ~ end ~ "` in site/snippets/" ~ file) }}
{%- endif %}
```{{ language | default(value="rust") }}
{{ lines | join(sep="
") }}
```
