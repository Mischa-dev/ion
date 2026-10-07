//! Page screenshots: where to scroll for a full-page capture, and what to
//! call the file.

/// Tallest full-page capture, in CSS pixels. Longer pages are cut off here so
/// one screenshot cannot use gigabytes of memory.
pub const MAX_PAGE_HEIGHT: f64 = 20_000.0;

/// Sizes a capture can be planned from: a positive viewport and a real page
/// height (NaN fails both checks).
fn usable(total: f64, viewport: f64) -> bool {
    viewport.is_finite() && viewport > 0.0 && total.is_finite()
}

/// Scroll positions (CSS pixels from the top) that together cover a page of
/// `total` height through a viewport of `viewport` height. The last tile is
/// aligned to the bottom, so tiles may overlap but never leave a gap. Pages
/// taller than `MAX_PAGE_HEIGHT` are covered up to that height.
pub fn tile_offsets(total: f64, viewport: f64) -> Vec<f64> {
    if !usable(total, viewport) {
        return vec![0.0];
    }
    let total = total.clamp(viewport, MAX_PAGE_HEIGHT.max(viewport));
    let last = total - viewport;
    let mut offsets = Vec::new();
    let mut y = 0.0;
    while y < last {
        offsets.push(y);
        y += viewport;
    }
    offsets.push(last);
    offsets
}

/// The height a full-page capture of `total` CSS pixels ends up with.
pub fn captured_height(total: f64, viewport: f64) -> f64 {
    if !usable(total, viewport) {
        return viewport.max(0.0);
    }
    total.clamp(viewport, MAX_PAGE_HEIGHT.max(viewport))
}

/// File name for a screenshot: "Screenshot example.com 2026-10-07 11.05.03.png".
/// `stamp` is the local date and time, already formatted; pages without a
/// host are called "page".
pub fn file_name(url: &str, stamp: &str) -> String {
    let host = crate::display_host(url).unwrap_or_else(|| "page".to_owned());
    let stamp = stamp.trim();
    let name = if stamp.is_empty() {
        format!("Screenshot {host}.png")
    } else {
        format!("Screenshot {host} {stamp}.png")
    };
    crate::downloads::sanitize_file_name(&name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_page_is_one_tile() {
        assert_eq!(tile_offsets(500.0, 800.0), vec![0.0]);
        assert_eq!(tile_offsets(800.0, 800.0), vec![0.0]);
        assert_eq!(captured_height(500.0, 800.0), 800.0);
    }

    #[test]
    fn last_tile_is_bottom_aligned() {
        assert_eq!(tile_offsets(2000.0, 800.0), vec![0.0, 800.0, 1200.0]);
        assert_eq!(tile_offsets(1600.0, 800.0), vec![0.0, 800.0]);
        assert_eq!(captured_height(2000.0, 800.0), 2000.0);
    }

    #[test]
    fn huge_pages_stop_at_the_limit() {
        let offsets = tile_offsets(1_000_000.0, 1000.0);
        assert_eq!(offsets.first(), Some(&0.0));
        assert_eq!(offsets.last(), Some(&(MAX_PAGE_HEIGHT - 1000.0)));
        assert_eq!(offsets.len(), 20);
        assert_eq!(captured_height(1_000_000.0, 1000.0), MAX_PAGE_HEIGHT);
    }

    #[test]
    fn bad_sizes_fall_back_to_one_tile() {
        assert_eq!(tile_offsets(f64::NAN, 800.0), vec![0.0]);
        assert_eq!(tile_offsets(2000.0, 0.0), vec![0.0]);
    }

    #[test]
    fn names_carry_host_and_time() {
        assert_eq!(
            file_name("https://www.example.com/a", "2026-10-07 11.05.03"),
            "Screenshot example.com 2026-10-07 11.05.03.png"
        );
        assert_eq!(file_name("about:blank", ""), "Screenshot page.png");
        // Characters that are unsafe in file names are replaced.
        assert_eq!(
            file_name("file:///tmp/x.html", "2026-10-07 11:05"),
            "Screenshot page 2026-10-07 11_05.png"
        );
    }
}
