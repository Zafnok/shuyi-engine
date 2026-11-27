{#- Appendix A's table (ticket 3413): a row per `[[term]]` of
    `site/data/engine-terms.toml`, Shuyi's words first, then the words of
    five engines. `cargo xtask llms` writes the same table in the twins,
    and `shuyi docs` reads it into its corpus. It takes no argument. -#}
{%- set terms = load_data(path="data/engine-terms.toml", format="toml") %}
| Shuyi Engine | Godot | Unity | Bevy | GameMaker | PICO-8 |
| ------------ | ----- | ----- | ---- | --------- | ------ |
{% for term in terms.term -%}
| {{ term.shuyi }} | {{ term.godot }} | {{ term.unity }} | {{ term.bevy }} | {{ term.gamemaker }} | {{ term.pico8 }} |
{% endfor -%}
