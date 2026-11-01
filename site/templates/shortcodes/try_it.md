{#- A picture of the tutorial's game (ticket 2818): the try `name` of
    `site/tutorial/tries.toml`, which `cargo xtask tutorial-tries` renders
    into `static/tutorial-tries/<name>/`. With no edits, what the reader
    should see. With edits, each edit as a diff, then the picture before and
    the picture after. A name or a picture that isn't there fails the build.
    `cargo xtask llms` expands it too. -#}
{%- set all = load_data(path="tutorial/tries.toml", format="toml") -%}
{%- set one = all[name] -%}
{%- set folder = "tutorial-tries/" ~ name ~ "/" -%}
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
{%- else %}
<figure class="try-one"><img src="{{ get_url(path=folder ~ "before.png") | safe }}" width="{{ before.width }}" height="{{ before.height }}" loading="lazy" alt="{{ one.alt }}"></figure>
{%- endif %}
