use ammonia::Builder;

/// ADR D8: Pipeline de Sanitização Editorial Estrita
/// Previne qualquer injeção de Stored XSS ou Mutation XSS na redação
pub fn sanitizar_html_editorial(raw_html: &str) -> String {
    let mut builder = Builder::default();

    // Tags permitidas para artigos editoriais
    builder.tags([
        "p", "h2", "h3", "h4", "blockquote", "ul", "ol", "li",
        "strong", "em", "b", "i", "u", "s", "hr", "br",
        "a", "img", "figure", "figcaption",
    ]);

    // Esquemas seguros de URLs
    builder.url_schemes(["http", "https", "mailto"]);

    // Restrição estrita de atributos
    builder.add_tag_attributes("a", &["href", "title", "target", "rel"]);
    builder.add_tag_attributes("img", &["src", "alt", "title", "loading", "width", "height"]);

    // Forçar rel seguro para mitigar tabnabbing
    builder.link_rel(Some("noopener noreferrer nofollow"));

    builder.clean(raw_html).to_string()
}
