//! Embeds `files/`, leaving its notes out.

fn main() {
    shuyi_embed::Table::new("files")
        .skip("**/notes.txt")
        .write();
}
