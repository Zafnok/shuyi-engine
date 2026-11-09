{#- A passage only a newcomer needs (ticket 2825, ADR-0090 §5), between
    `{% newcomer(to="rust", topic="`match`") %}` and `{% end %}`, each on a
    line of its own. `to` is one of the four audiences, `topic` a few words
    of inline Markdown saying what it explains. With no JavaScript every
    passage shows, under its label; 2826's filter folds the ones a reader
    knows. `cargo xtask check-book` checks the marks, and `cargo xtask llms`
    writes each as a blockquote in the twins. -#}
{%- if to is not defined -%}
{{ throw(message="newcomer: no `to`: programming, games, rust or shuyi") }}
{%- elif to == "programming" -%}{%- set label = "New to programming?" -%}
{%- elif to == "games" -%}{%- set label = "New to making games?" -%}
{%- elif to == "rust" -%}{%- set label = "New to Rust?" -%}
{%- elif to == "shuyi" -%}{%- set label = "New to Shuyi?" -%}
{%- else -%}
{{ throw(message='newcomer: to="' ~ to ~ '" is not one of programming, games, rust or shuyi') }}
{%- endif -%}
{%- if topic is not defined or topic | trim == "" -%}
{{ throw(message="newcomer: no `topic`: a few words saying what the passage explains") }}
{%- endif %}
<aside class="newcomer" data-newcomer="{{ to }}">
<p class="newcomer-label">{{ label }} <span>{{ topic | markdown(inline=true) | safe }}</span></p>

{{ body }}

</aside>
