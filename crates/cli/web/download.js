// Saving a file on the web, which has no files of its own: a miniquad plugin
// that offers the browser a download. This file is Shuyi Engine's own code,
// unlike the vendored plugins next to it.
//
// The Rust side is crates/app/src/sessions/web.rs, which declares exactly the
// `shuyi_download` function below (a test of shuyi-cli compares the two).
(function () {
    "use strict";

    // Keep equal to PLUGIN_VERSION in crates/app/src/sessions/web.rs.
    var VERSION = 1;

    function register_plugin(importObject) {
        // Offers a file called the UTF-8 text at name_ptr, holding the bytes
        // at text_ptr. Both are read now: wasm memory may move later.
        importObject.env.shuyi_download = function (name_ptr, name_len, text_ptr, text_len) {
            var name = new TextDecoder().decode(new Uint8Array(wasm_memory.buffer, name_ptr, name_len));
            var text = new Uint8Array(wasm_memory.buffer, text_ptr, text_len).slice();
            var url = URL.createObjectURL(new Blob([text], { type: "text/plain" }));
            var link = document.createElement("a");
            link.href = url;
            link.download = name;
            document.body.appendChild(link);
            link.click();
            link.remove();
            setTimeout(function () {
                URL.revokeObjectURL(url);
            }, 0);
        };
    }

    miniquad_add_plugin({
        register_plugin: register_plugin,
        name: "shuyi_download",
        version: VERSION
    });
})();
