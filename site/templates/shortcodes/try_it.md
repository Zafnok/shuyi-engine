{#- A picture of the project's game or of Moon Garden (tickets 2818, 2819): the try `name` of
    `site/tutorial/tries.toml`, which `cargo xtask tutorial-tries` renders
    into `static/tutorial-tries/<name>/`. With no edits, what the reader
    should see. With edits, each edit as a diff, then the picture before and
    the picture after. A name or a picture that isn't there fails the build.
    `cargo xtask llms` expands it too.

    Ticket 2820: on its own step's page, a try whose edits are all of files
    that page's playground holds (`extra.files`, each the path after the
    first `files/` of the edit's file) gets a "Try this here" button, which
    `playground.js` shows and runs. A try that edits Rust says it needs a
    build on the reader's own machine. -#}
{%- set all = load_data(path="tutorial/tries.toml", format="toml") -%}
{%- set one = all[name] -%}
{%- set folder = "tutorial-tries/" ~ name ~ "/" -%}
{%- set_global rust = false -%}
{%- set_global here = one.edits is defined and one.edits | length > 0 and page is defined and page.extra.files is defined and page.slug == one.step -%}
{%- if one.edits is defined -%}
{%- for edit in one.edits -%}
{%- if edit.file is ending_with(".rs") %}{% set_global rust = true %}{% endif -%}
{%- if here -%}
{%- if not edit.file is containing("files/") -%}
{%- set_global here = false -%}
{%- elif not page.extra.files is containing(edit.file | split(pat="files/") | slice(start=1) | join(sep="files/")) -%}
{%- set_global here = false -%}
{%- endif -%}
{%- endif -%}
{%- endfor -%}
{%- endif -%}
{%- set before = get_image_metadata(path=folder ~ "before.png") -%}
{%- if one.edits is defined and one.edits | length > 0 -%}
{%- set after = get_image_metadata(path=folder ~ "after.png") -%}
{%- for edit in one.edits %}
In `{{ edit.file }}`:

```diff
{% for line in edit.find | split(pat="
") -%}
- {{ line }}
{% endfor -%}
{% for line in edit.replace | split(pat="
") -%}
+ {{ line }}
{% endfor -%}
```
{% endfor %}
<div class="try-pair">
<figure><img src="{{ get_url(path=folder ~ "before.png") | safe }}" width="{{ before.width }}" height="{{ before.height }}" loading="lazy" alt="{{ one.alt }}"><figcaption>Before</figcaption></figure>
<figure><img src="{{ get_url(path=folder ~ "after.png") | safe }}" width="{{ after.width }}" height="{{ after.height }}" loading="lazy" alt="{{ one.after }}"><figcaption>After</figcaption></figure>
</div>
{%- if here and not rust %}
<p class="try-here" hidden><button type="button" class="button" data-edits="{{ one.edits | json_encode() | escape }}">Try this here</button> The playground at the top of this page opens with this change made, and runs it.</p>
{%- elif rust %}
<p class="try-rust">This change is to the Rust code: try it on your own machine, where <code>cargo run</code> builds the game again with it. A web page can't build Rust.</p>
{%- endif %}
{%- else %}
<figure class="try-one"><img src="{{ get_url(path=folder ~ "before.png") | safe }}" width="{{ before.width }}" height="{{ before.height }}" loading="lazy" alt="{{ one.alt }}"></figure>
{%- endif %}
