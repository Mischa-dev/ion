use ion_reader::{Style, extract, render};

fn long_prose() -> String {
    "Reader mode should preserve meaningful prose while removing active page content, navigation, advertisements, and tracking elements. ".repeat(4)
}

#[test]
fn extraction_and_rendering_form_a_safe_end_to_end_pipeline() {
    let prose = long_prose();
    let html = format!(
        r#"<!doctype html><html><head>
      <title>A &amp; B &lt;Guide&gt;</title>
      <meta name="author" content="Ada &amp; Lin">
      <meta property="og:site_name" content="Example News">
    </head><body><nav>Do not keep this navigation</nav><article>
      <h1>A &amp; B &lt;Guide&gt;</h1>
      <p>{prose}</p>
      <a href="/next?x=1&amp;y=2" onclick="steal()">Continue</a>
      <script>alert('unsafe')</script>
    </article></body></html>"#
    );

    let article = extract(&html, "https://example.com/stories/current").unwrap();
    let page = render(&article, &Style::default());

    assert_eq!(article.title, "A & B <Guide>");
    assert_eq!(article.byline, "Ada & Lin");
    assert_eq!(article.site, "Example News");
    assert!(
        article
            .content
            .contains("https://example.com/next?x=1&amp;y=2")
    );
    assert!(!article.content.contains("onclick"));
    assert!(!page.contains("alert('unsafe')"));
    assert!(page.contains("default-src 'none'"));
    assert!(page.contains("<title>A &amp; B &lt;Guide&gt;</title>"));
}

#[test]
fn relative_media_urls_are_resolved_and_dangerous_schemes_are_removed() {
    let prose = long_prose();
    let html = format!(
        r#"<main><p>{prose}</p>
      <img src="../media/photo.jpg" alt="Photo" onerror="bad()">
      <a href="javascript:bad()">unsafe link</a>
    </main>"#
    );

    let article = extract(&html, "https://example.com/news/2026/story").unwrap();

    assert!(
        article
            .content
            .contains("src=\"https://example.com/news/media/photo.jpg\"")
    );
    assert!(article.content.contains(">unsafe link</a>"));
    assert!(!article.content.contains("javascript:"));
    assert!(!article.content.contains("onerror"));
}

#[test]
fn short_or_navigation_only_pages_do_not_offer_reader_mode() {
    assert!(
        extract(
            "<main><p>Just a short status.</p></main>",
            "https://example.com"
        )
        .is_none()
    );
    assert!(
        extract(
            &format!("<nav>{}</nav>", long_prose()),
            "https://example.com"
        )
        .is_none()
    );
}

#[test]
fn renderer_clamps_font_size_and_rejects_css_injection() {
    let article = extract(
        &format!("<article><p>{}</p></article>", long_prose()),
        "https://example.com/article",
    )
    .unwrap();
    let style = Style {
        background: "red; } body { display:none".into(),
        font_size: 500,
        serif: false,
        ..Style::default()
    };

    let page = render(&article, &style);

    assert!(page.contains("--bg: inherit;"));
    assert!(page.contains("--size: 32px;"));
    assert!(page.contains("--font: system-ui"));
    assert!(!page.contains("display:none"));
}
