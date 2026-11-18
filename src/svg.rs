//! SvgBuilder: a small string-based SVG writer with one number formatter, one
//! escaper, deterministic ids and a few safe building blocks.
//!
//! Rules it enforces by construction: hex colors plus separate opacity, no
//! external references, no scripts, no images. (The one filter, film grain,
//! is added explicitly by the caller.)

/// Format a number with at most two decimals, no trailing zeros, never `-0`,
/// never scientific notation. A non-finite value prints as `0`: the linter,
/// not the writer, is where bad numbers get caught.
pub fn fmt(value: f64) -> String {
    if !value.is_finite() {
        return "0".to_string();
    }
    let text = format!("{:.2}", (value * 100.0).round() / 100.0);
    let trimmed = text.trim_end_matches('0').trim_end_matches('.');
    if trimmed.is_empty() || trimmed == "-0" {
        "0".to_string()
    } else {
        trimmed.to_string()
    }
}

/// Escape text or an attribute value: `& < > " '`.
pub fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            other => out.push(other),
        }
    }
    out
}

/// Is this `#rrggbb`?
pub fn is_hex_color(text: &str) -> bool {
    text.len() == 7 && text.starts_with('#') && text[1..].chars().all(|c| c.is_ascii_hexdigit())
}

/// How a gradient's coordinates are read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Units {
    /// Fractions of the shape being filled (so one gradient can fill many shapes).
    ObjectBoundingBox,
    /// Canvas pixels.
    UserSpaceOnUse,
}

impl Units {
    fn name(self) -> &'static str {
        match self {
            Units::ObjectBoundingBox => "objectBoundingBox",
            Units::UserSpaceOnUse => "userSpaceOnUse",
        }
    }
}

/// One gradient stop: a position (0 to 1), a hex color and an opacity.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stop<'a> {
    pub offset: f64,
    pub color: &'a str,
    pub opacity: f64,
}

impl<'a> Stop<'a> {
    pub fn new(offset: f64, color: &'a str, opacity: f64) -> Self {
        Stop {
            offset,
            color,
            opacity,
        }
    }
}

/// Build `<name a="b" ...>inner</name>` (or a self-closing tag with no `inner`).
/// Attribute values are escaped.
pub fn tag(name: &str, attrs: &[(&str, String)], inner: Option<&str>) -> String {
    let mut out = format!("<{name}");
    for (key, value) in attrs {
        out.push_str(&format!(" {key}=\"{}\"", escape(value)));
    }
    match inner {
        Some(inner) => out.push_str(&format!(">{inner}</{name}>")),
        None => out.push_str("/>"),
    }
    out
}

fn with_extra(
    mut base: Vec<(&'static str, String)>,
    extra: &[(&str, String)],
) -> Vec<(String, String)> {
    let mut all: Vec<(String, String)> = base.drain(..).map(|(k, v)| (k.to_string(), v)).collect();
    all.extend(extra.iter().map(|(k, v)| ((*k).to_string(), v.clone())));
    all
}

fn tag_owned(name: &str, attrs: Vec<(String, String)>, inner: Option<&str>) -> String {
    let borrowed: Vec<(&str, String)> =
        attrs.iter().map(|(k, v)| (k.as_str(), v.clone())).collect();
    tag(name, &borrowed, inner)
}

pub fn rect(x: f64, y: f64, w: f64, h: f64, extra: &[(&str, String)]) -> String {
    let base = vec![
        ("x", fmt(x)),
        ("y", fmt(y)),
        ("width", fmt(w)),
        ("height", fmt(h)),
    ];
    tag_owned("rect", with_extra(base, extra), None)
}

pub fn circle(cx: f64, cy: f64, r: f64, extra: &[(&str, String)]) -> String {
    tag_owned(
        "circle",
        with_extra(vec![("cx", fmt(cx)), ("cy", fmt(cy)), ("r", fmt(r))], extra),
        None,
    )
}

pub fn ellipse(cx: f64, cy: f64, rx: f64, ry: f64, extra: &[(&str, String)]) -> String {
    let base = vec![
        ("cx", fmt(cx)),
        ("cy", fmt(cy)),
        ("rx", fmt(rx)),
        ("ry", fmt(ry)),
    ];
    tag_owned("ellipse", with_extra(base, extra), None)
}

pub fn path(d: &str, extra: &[(&str, String)]) -> String {
    tag_owned("path", with_extra(vec![("d", d.to_string())], extra), None)
}

/// A text element (not used by the background; ready for later milestones).
pub fn text(x: f64, y: f64, content: &str, extra: &[(&str, String)]) -> String {
    tag_owned(
        "text",
        with_extra(vec![("x", fmt(x)), ("y", fmt(y))], extra),
        Some(&escape(content)),
    )
}

/// A `<style>` element (not used yet). The CSS is escaped as text.
pub fn style(css: &str) -> String {
    format!("<style>{}</style>", escape(css))
}

/// Catmull-Rom spline through `points`, written as cubic Bezier segments.
/// Open paths end exactly on the first and last point; closed ones end with `Z`.
pub fn smooth_path(points: &[(f64, f64)], closed: bool) -> String {
    let n = points.len();
    if n == 0 {
        return String::new();
    }
    let mut d = format!("M{} {}", fmt(points[0].0), fmt(points[0].1));
    if n == 1 {
        return d;
    }
    let at = |i: isize| -> (f64, f64) {
        if closed {
            points[i.rem_euclid(n as isize) as usize]
        } else {
            points[i.clamp(0, n as isize - 1) as usize]
        }
    };
    let segments = if closed { n } else { n - 1 };
    for i in 0..segments as isize {
        let (p0, p1, p2, p3) = (at(i - 1), at(i), at(i + 1), at(i + 2));
        let c1 = (p1.0 + (p2.0 - p0.0) / 6.0, p1.1 + (p2.1 - p0.1) / 6.0);
        let c2 = (p2.0 - (p3.0 - p1.0) / 6.0, p2.1 - (p3.1 - p1.1) / 6.0);
        d.push_str(&format!(
            "C{} {} {} {} {} {}",
            fmt(c1.0),
            fmt(c1.1),
            fmt(c2.0),
            fmt(c2.1),
            fmt(p2.0),
            fmt(p2.1)
        ));
    }
    if closed {
        d.push('Z');
    }
    d
}

/// Builds one SVG document.
#[derive(Debug, Clone)]
pub struct SvgBuilder {
    width: f64,
    height: f64,
    title: String,
    desc: String,
    defs: String,
    body: String,
    next_id: u32,
}

impl SvgBuilder {
    pub fn new(width: f64, height: f64, title: &str, desc: &str) -> Self {
        SvgBuilder {
            width,
            height,
            title: title.to_string(),
            desc: desc.to_string(),
            defs: String::new(),
            body: String::new(),
            next_id: 0,
        }
    }

    pub fn width(&self) -> f64 {
        self.width
    }

    pub fn height(&self) -> f64 {
        self.height
    }

    /// A fresh id like `sky1`, `sky2`... (one counter for the whole document,
    /// so ids never collide, whatever the prefix).
    pub fn new_id(&mut self, prefix: &str) -> String {
        self.next_id += 1;
        format!("{prefix}{}", self.next_id)
    }

    fn stops(stops: &[Stop]) -> String {
        stops
            .iter()
            .map(|s| {
                debug_assert!(
                    is_hex_color(s.color),
                    "stop color must be #rrggbb: {}",
                    s.color
                );
                let attrs = [
                    ("offset", fmt(s.offset)),
                    ("stop-color", s.color.to_string()),
                    ("stop-opacity", fmt(s.opacity)),
                ];
                tag("stop", &attrs, None)
            })
            .collect()
    }

    pub fn linear_gradient(
        &mut self,
        id: &str,
        from: (f64, f64),
        to: (f64, f64),
        units: Units,
        stops: &[Stop],
    ) {
        let attrs = [
            ("id", id.to_string()),
            ("x1", fmt(from.0)),
            ("y1", fmt(from.1)),
            ("x2", fmt(to.0)),
            ("y2", fmt(to.1)),
            ("gradientUnits", units.name().to_string()),
        ];
        self.defs
            .push_str(&tag("linearGradient", &attrs, Some(&Self::stops(stops))));
        self.defs.push('\n');
    }

    pub fn radial_gradient(
        &mut self,
        id: &str,
        center: (f64, f64),
        radius: f64,
        units: Units,
        stops: &[Stop],
    ) {
        let attrs = [
            ("id", id.to_string()),
            ("cx", fmt(center.0)),
            ("cy", fmt(center.1)),
            ("r", fmt(radius)),
            ("gradientUnits", units.name().to_string()),
        ];
        self.defs
            .push_str(&tag("radialGradient", &attrs, Some(&Self::stops(stops))));
        self.defs.push('\n');
    }

    pub fn clip_path(&mut self, id: &str, inner: &str) {
        self.defs
            .push_str(&tag("clipPath", &[("id", id.to_string())], Some(inner)));
        self.defs.push('\n');
    }

    /// Add a ready-made definition (used for the single film-grain filter).
    pub fn add_def(&mut self, fragment: &str) {
        self.defs.push_str(fragment);
        self.defs.push('\n');
    }

    /// Add a ready-made element to the drawing.
    pub fn add(&mut self, fragment: &str) {
        self.body.push_str(fragment);
        self.body.push('\n');
    }

    /// Draw a `<g>` with the given attributes (id, class, transform, opacity...).
    pub fn group(&mut self, attrs: &[(&str, String)], draw: impl FnOnce(&mut SvgBuilder)) {
        let open = tag("g", attrs, Some("")).replace("></g>", ">");
        self.body.push_str(&open);
        self.body.push('\n');
        draw(self);
        self.body.push_str("</g>\n");
    }

    /// The finished document.
    pub fn finish(self) -> String {
        let mut out = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {w} {h}\" width=\"{w}\" height=\"{h}\" role=\"img\">\n",
            w = fmt(self.width),
            h = fmt(self.height)
        );
        out.push_str(&format!(
            "<title>{}</title>\n<desc>{}</desc>\n",
            escape(&self.title),
            escape(&self.desc)
        ));
        out.push_str("<defs>\n");
        out.push_str(&self.defs);
        out.push_str("</defs>\n");
        out.push_str(&self.body);
        out.push_str("</svg>\n");
        out
    }
}
