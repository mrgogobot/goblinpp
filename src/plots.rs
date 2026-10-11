//! Exact array plotting evidence and a versioned, deterministic renderer.
//! Historical FITS renderers deliberately remain unchanged.
use crate::error::{GoblinError, Result};
use crate::evaluator::Value;
use crate::hashing::{hash_canonical_json, sha256_bytes};
use crate::output::{Canvas, GeneratedOutput};
use crate::quantity::{Dimension, format_dimension};
use serde::{Deserialize, Serialize};
use serde_json::json;

pub const POLICY: &str = "goblin.array-plots.v2";
pub const LEGACY_POLICY: &str = "goblin.array-plots.v1";
pub const MAX_EVIDENCE_BYTES: usize = 16 * 1024 * 1024;
const WIDTH: usize = 960;
const HEIGHT: usize = 600;
const LEFT: usize = 200;
const RIGHT: usize = 870;
const TOP: usize = 100;
const BOTTOM: usize = 510;
const BLACK: [u8; 3] = [35, 40, 48];
const BLUE: [u8; 3] = [36, 105, 175];
const GRID: [u8; 3] = [220, 225, 232];

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Axis {
    pub label: String,
    pub dimension: Dimension,
    /// IEEE-754 bits: signed zero and every input bit survive JSON serialization.
    pub values_bits: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Input {
    pub figure: String,
    pub kind: String,
    pub title: String,
    pub x: Axis,
    pub y: Option<Axis>,
    pub bins: Option<usize>,
}

pub fn is_function(name: &str) -> bool {
    matches!(name, "plot_line" | "plot_scatter" | "plot_histogram")
}
pub fn requires_policy(receipt: &serde_json::Value) -> bool {
    let version = receipt["goblin_version"].as_str().unwrap_or("");
    !(version.starts_with("0.0.")
        || version
            .strip_prefix("0.1.0-alpha.")
            .and_then(|n| n.parse::<u32>().ok())
            .is_some_and(|n| n < 30))
}
pub fn frozen_policy_matches(receipt: &serde_json::Value) -> bool {
    receipt
        .get("plot_policy")
        .map_or(!requires_policy(receipt), supports_policy)
}
pub fn supports_policy(value: &serde_json::Value) -> bool {
    matches!(value.as_str(), Some(POLICY | LEGACY_POLICY))
}
pub fn program_uses_plots(program: &crate::ast::Program) -> bool {
    fn visit(value: &serde_json::Value) -> bool {
        value.as_array().is_some_and(|items| {
            (items.first().is_some_and(|v| v == "call")
                && items
                    .get(1)
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(is_function))
                || items.iter().any(visit)
        })
    }
    visit(&program.canonical())
}
pub fn require_arity(name: &str, count: usize) -> Result<()> {
    let expected = if name == "plot_histogram" { 5 } else { 6 };
    if count != expected {
        return Err(GoblinError::artifact(format!(
            "{name}() requires {expected} arguments; received {count}."
        )));
    }
    Ok(())
}
fn fail(message: impl Into<String>) -> GoblinError {
    GoblinError::artifact(message)
}
fn axis(value: &Value, label: &Value) -> Result<Axis> {
    let Value::Array(items) = value else {
        return Err(fail("Plot data must be a numeric array."));
    };
    if items.is_empty() || items.len() > crate::evaluator::MAX_ARRAY_ITEMS {
        return Err(fail("Plot arrays must contain 1..100000 values."));
    }
    let dimension = items[0].quantity("plot array")?.dimension;
    let mut bits = Vec::with_capacity(items.len());
    for item in items {
        let q = item.quantity("plot array")?;
        if q.dimension != dimension || !q.value_si.is_finite() {
            return Err(fail(
                "Each plot axis requires finite numbers with matching dimensions. No rows are silently dropped.",
            ));
        }
        bits.push(format!("{:016x}", q.value_si.to_bits()));
    }
    Ok(Axis {
        label: label.text("plot axis label")?.into(),
        dimension,
        values_bits: bits,
    })
}
pub fn call(name: &str, args: &[Value]) -> Result<Vec<GeneratedOutput>> {
    require_arity(name, args.len())?;
    let histogram = name == "plot_histogram";
    let bins = if histogram {
        let q = args[2].quantity("histogram bin count")?;
        if q.dimension != crate::quantity::DIMENSIONLESS
            || !(2.0..=500.0).contains(&q.value_si)
            || q.value_si.fract() != 0.0
        {
            return Err(fail(
                "Histogram bin count must be a dimensionless integer in 2..500.",
            ));
        }
        Some(q.value_si as usize)
    } else {
        None
    };
    build(&Input {
        figure: args[0].text(name)?.into(),
        kind: name.strip_prefix("plot_").unwrap().into(),
        title: args[3].text("plot title")?.into(),
        x: axis(&args[1], &args[4])?,
        y: if histogram {
            None
        } else {
            Some(axis(&args[2], &args[5])?)
        },
        bins,
    })
}
fn decode(axis: &Axis) -> Result<Vec<f64>> {
    if axis.values_bits.is_empty()
        || axis.values_bits.len() > 100000
        || axis.dimension.iter().any(|p| !(-64..=64).contains(p))
    {
        return Err(fail(
            "Invalid plot axis size or dimension exponent (supported -64..64).",
        ));
    }
    axis.values_bits
        .iter()
        .map(|bits| {
            if bits.len() != 16
                || !bits
                    .bytes()
                    .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
            {
                return Err(fail(
                    "Plot values require canonical 16-digit lowercase f64 bits.",
                ));
            }
            let v = f64::from_bits(
                u64::from_str_radix(bits, 16).map_err(|_| fail("Invalid plot bits."))?,
            );
            if !v.is_finite() {
                return Err(fail("Plot evidence contains a nonfinite value."));
            }
            Ok(v)
        })
        .collect()
}
fn labelled(axis: &Axis) -> String {
    format!("{} [{}]", axis.label, format_dimension(axis.dimension))
}
fn label(text: &str, maximum: usize, png: bool) -> Result<()> {
    if text
        .chars()
        .map(|c| if c.is_ascii() { 1 } else { 2 })
        .sum::<usize>()
        > maximum
        || text.chars().any(char::is_control)
        || text
            .chars()
            .any(|c| matches!(c, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}'))
    {
        return Err(fail(format!(
            "Plot label must be a single line of at most {maximum} characters without controls or bidirectional overrides."
        )));
    }
    if png && text.chars().any(|c| !supported(c)) {
        return Err(fail(
            "PNG labels support Latin letters, digits and common scientific punctuation. Use SVG for Unicode labels; Goblin++ will not silently replace glyphs.",
        ));
    }
    Ok(())
}
fn bounds(values: &[f64]) -> Result<(f64, f64)> {
    let mut lo = values.iter().copied().fold(f64::INFINITY, f64::min);
    let mut hi = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if lo == hi {
        let pad = lo.abs().max(1.0) * 0.05;
        let a = lo - pad;
        let b = hi + pad;
        if a.is_finite() {
            lo = a;
        }
        if b.is_finite() {
            hi = b;
        }
    }
    if lo >= hi || !(hi - lo).is_finite() || hi - lo <= 0.0 {
        return Err(fail(
            "Plot axis span cannot be represented safely. Rescale the data explicitly; no clipping or hidden transform is applied.",
        ));
    }
    Ok((lo, hi))
}
fn interpolate((lo, hi): (f64, f64), t: f64) -> f64 {
    if t == 0.0 {
        lo
    } else if t == 1.0 {
        hi
    } else {
        (lo * (1.0 - t) + hi * t).clamp(lo, hi)
    }
}
fn position(v: f64, bounds: (f64, f64), start: usize, end: usize) -> f64 {
    start as f64 + (v - bounds.0) / (bounds.1 - bounds.0) * (end - start) as f64
}
fn ticks(bounds: (f64, f64)) -> Vec<f64> {
    let mut values: Vec<f64> = (0..=4)
        .map(|i| interpolate(bounds, i as f64 / 4.0))
        .collect();
    values.dedup_by(|a, b| *a == *b);
    values
}
// Display ticks only are explicitly rounded. Authoritative inputs/bounds remain bits.
fn tick(v: f64, precision: usize) -> String {
    if v == 0.0 {
        "0".into()
    } else {
        format!("{v:.precision$e}")
    }
}
fn labelled_ticks(bounds: (f64, f64)) -> Vec<(f64, String)> {
    let values = ticks(bounds);
    let mut labels = values.iter().map(|v| tick(*v, 3)).collect::<Vec<_>>();
    if labels.windows(2).any(|w| w[0] == w[1]) {
        labels = values.iter().map(|v| tick(*v, 16)).collect();
    }
    values.into_iter().zip(labels).collect()
}
fn tick_scale(bounds: (f64, f64)) -> usize {
    if labelled_ticks(bounds).iter().all(|(_, s)| s.len() <= 11) {
        2
    } else {
        1
    }
}
fn bits(v: f64) -> String {
    format!("{:016x}", v.to_bits())
}
struct Layout {
    x: Vec<f64>,
    y: Vec<f64>,
    xb: (f64, f64),
    yb: (f64, f64),
    edges: Vec<f64>,
    counts: Vec<usize>,
    xl: String,
    yl: String,
}
fn layout(input: &Input, png: bool) -> Result<Layout> {
    let x = decode(&input.x)?;
    let xl = format!("X: {}", labelled(&input.x));
    label(&input.title, 70, png)?;
    label(&xl, 55, png)?;
    let xb = bounds(&x)?;
    let mut edges = Vec::new();
    let mut counts = Vec::new();
    let (y, yb, yl) = match input.kind.as_str() {
        "line" | "scatter" => {
            if input.bins.is_some() {
                return Err(fail("Line/scatter cannot contain histogram bins."));
            }
            let axis = input
                .y
                .as_ref()
                .ok_or_else(|| fail("Plot y axis missing."))?;
            let y = decode(axis)?;
            if y.len() != x.len() || (input.kind == "line" && x.len() < 2) {
                return Err(fail(
                    "x/y arrays must have matching lengths; line plots require at least two points.",
                ));
            }
            let yb = bounds(&y)?;
            (y, yb, format!("Y: {}", labelled(axis)))
        }
        "histogram" => {
            if input.y.is_some() {
                return Err(fail("Histogram must not have an input y axis."));
            }
            let bins = input
                .bins
                .filter(|n| (2..=500).contains(n))
                .ok_or_else(|| fail("Invalid histogram bin count."))?;
            edges = (0..=bins)
                .map(|i| interpolate(xb, i as f64 / bins as f64))
                .collect();
            if edges.windows(2).any(|w| w[0] >= w[1]) {
                return Err(fail(
                    "Histogram edges collapse at this floating-point scale. Use fewer bins or explicitly rescale.",
                ));
            }
            counts = vec![0; bins];
            for value in &x {
                let bin = edges
                    .partition_point(|edge| edge <= value)
                    .saturating_sub(1)
                    .min(bins - 1);
                counts[bin] += 1;
            }
            let maximum = *counts.iter().max().unwrap() as f64;
            (Vec::new(), (0.0, maximum), "Y: count [1]".into())
        }
        _ => return Err(fail("Unsupported array plot kind.")),
    };
    label(&yl, 55, png)?;
    Ok(Layout {
        x,
        y,
        xb,
        yb,
        edges,
        counts,
        xl,
        yl,
    })
}
pub fn build(input: &Input) -> Result<Vec<GeneratedOutput>> {
    build_with_policy(input, POLICY)
}

// v1 must retain its original bytes for historical reconstruction, not acquire
// new metadata retroactively. New executions always use v2.
fn build_with_policy(input: &Input, policy: &str) -> Result<Vec<GeneratedOutput>> {
    let extension = crate::output::extension(&input.figure)?;
    let png = match extension.as_str() {
        "png" => true,
        "svg" => false,
        _ => return Err(fail("Array plots require .png or .svg.")),
    };
    let l = layout(input, png)?;
    let derived = json!({
        "width": WIDTH, "height": HEIGHT, "plot_rectangle": [LEFT,TOP,RIGHT,BOTTOM],
        "x_label":l.xl, "y_label":l.yl, "x_bounds_bits":[bits(l.xb.0),bits(l.xb.1)],
        "y_bounds_bits":[bits(l.yb.0),bits(l.yb.1)],
        "x_ticks":labelled_ticks(l.xb).into_iter().map(|(v,s)|json!({"value_bits":bits(v),"label":s})).collect::<Vec<_>>(),
        "y_ticks":labelled_ticks(l.yb).into_iter().map(|(v,s)|json!({"value_bits":bits(v),"label":s})).collect::<Vec<_>>(),
        "bin_edges_bits":l.edges.iter().map(|v|bits(*v)).collect::<Vec<_>>(), "bin_counts":l.counts,
        "tick_format":"scientific-3-decimal;16-if-distinct-values-would-share-labels;display-only", "axis_scale":"linear-SI",
        "tick_values":"bounded-convex-interpolation;numeric-duplicates-removed",
        "x_tick_font_scale":tick_scale(l.xb), "y_tick_font_scale":tick_scale(l.yb), "label_font_scale":2,
        "point_order":"as-supplied", "sampling":"none", "missing_values":"refuse",
        "histogram_intervals":"left-closed-right-open-final-right-inclusive",
        "constant_axis":"expand-5-percent-max-absolute-value-or-one; finite endpoints only",
        "colour":"#2469af", "line_width":1, "point_radius":1, "grid":true
    });
    let evidence_name = format!("{}.plot.json", input.figure);
    // This core is independent of the image, sidecar and final run receipt.
    let core = json!({"schema":policy,"input":input,"derived":derived});
    let mut bytes = render(input, &l, png);
    let mut document = core.clone();
    let mut figure_metadata = json!({"plot_policy":policy,"evidence_name":evidence_name});
    let mut evidence_metadata = json!({"plot_policy":policy,"figure_name":input.figure});
    if policy == POLICY {
        let manifest_hash = hash_canonical_json(&core)?;
        let provenance = crate::plot_provenance::Provenance::new(&manifest_hash, &evidence_name);
        bytes = crate::plot_provenance::embed(&bytes, png, &provenance)?;
        document["manifest_sha256"] = json!(manifest_hash);
        document["provenance_schema"] = json!(crate::plot_provenance::SCHEMA);
        for metadata in [&mut figure_metadata, &mut evidence_metadata] {
            metadata["manifest_sha256"] = json!(manifest_hash);
            metadata["provenance_schema"] = json!(crate::plot_provenance::SCHEMA);
        }
    }
    let figure_hash = sha256_bytes(&bytes);
    document["figure_sha256"] = json!(figure_hash);
    let evidence_bytes = serde_json::to_vec_pretty(&document)?;
    if evidence_bytes.len() > MAX_EVIDENCE_BYTES {
        return Err(fail("Plot evidence exceeds 16 MiB."));
    }
    let evidence_hash = sha256_bytes(&evidence_bytes);
    figure_metadata["evidence_sha256"] = json!(evidence_hash);
    evidence_metadata["figure_sha256"] = json!(figure_hash);
    Ok(vec![
        GeneratedOutput::new(
            &input.figure,
            if png { "image/png" } else { "image/svg+xml" },
            "array_plot",
            bytes,
            figure_metadata,
        )?,
        GeneratedOutput::new(
            &evidence_name,
            "application/json",
            "array_plot_evidence",
            evidence_bytes,
            evidence_metadata,
        )?,
    ])
}

/// Independently reconstruct both files and metadata, not merely their checksums.
pub fn verify_pair(
    figure: &[u8],
    sidecar: &[u8],
    figure_desc: &serde_json::Value,
    evidence_desc: &serde_json::Value,
) -> Result<()> {
    if sidecar.len() > MAX_EVIDENCE_BYTES || figure.len() > crate::output::MAX_OUTPUT_BYTES {
        return Err(fail("Plot evidence exceeds verification limits."));
    }
    let document: serde_json::Value = serde_json::from_slice(sidecar)?;
    if !supports_policy(&document["schema"]) {
        return Err(fail("Unsupported plot evidence policy."));
    }
    if document["schema"] == POLICY {
        verify_embedded_provenance(figure, &document)?;
    }
    let input: Input = serde_json::from_value(document["input"].clone())?;
    let outputs = build_with_policy(&input, document["schema"].as_str().unwrap())?;
    for (output, descriptor, actual) in [
        (&outputs[0], figure_desc, figure),
        (&outputs[1], evidence_desc, sidecar),
    ] {
        if output.bytes != actual
            || descriptor["name"] != output.name
            || descriptor["producer"] != output.producer
            || descriptor["metadata"] != output.metadata
            || descriptor["media_type"] != output.media_type
        {
            return Err(fail(
                "Plot data/settings, figure or evidence metadata do not reproduce exactly.",
            ));
        }
    }
    Ok(())
}

/// Checks the embedded link against the exact manifest core. This establishes
/// consistency, not authentication of an author or of a detached image.
pub fn verify_embedded_provenance(figure: &[u8], document: &serde_json::Value) -> Result<()> {
    if document["schema"] != POLICY {
        return Err(fail("Embedded plot provenance requires plot policy v2."));
    }
    let core = json!({"schema":document["schema"],"input":document["input"],"derived":document["derived"]});
    let digest = hash_canonical_json(&core)?;
    let name = document["input"]["figure"]
        .as_str()
        .ok_or_else(|| fail("Plot figure name missing."))?;
    let png = crate::output::extension(name)? == "png";
    let embedded = crate::plot_provenance::extract(figure, png)?;
    let expected = crate::plot_provenance::Provenance::new(&digest, &format!("{name}.plot.json"));
    if embedded != expected
        || document["manifest_sha256"] != digest
        || document["provenance_schema"] != crate::plot_provenance::SCHEMA
    {
        return Err(fail(
            "Plot provenance does not match its data/settings manifest.",
        ));
    }
    Ok(())
}

fn render(input: &Input, l: &Layout, png: bool) -> Vec<u8> {
    let mut drawing = Drawing::new(png);
    drawing.text(WIDTH / 2, 28, &input.title, 2, "middle");
    drawing.text(LEFT, 74, &l.yl, 2, "start");
    drawing.text((LEFT + RIGHT) / 2, 558, &l.xl, 2, "middle");
    drawing.text(
        (LEFT + RIGHT) / 2,
        583,
        "linear axes; all points; SI units; rounded tick labels only",
        1,
        "middle",
    );
    for (v, label) in labelled_ticks(l.xb) {
        let x = position(v, l.xb, LEFT, RIGHT).round() as usize;
        drawing.line(x, TOP, x, BOTTOM, GRID);
        drawing.text(x, BOTTOM + 12, &label, tick_scale(l.xb), "middle");
    }
    for (v, label) in labelled_ticks(l.yb) {
        let y = BOTTOM - (position(v, l.yb, 0, BOTTOM - TOP).round() as usize);
        drawing.line(LEFT, y, RIGHT, y, GRID);
        drawing.text(
            LEFT - 12,
            y.saturating_sub(7),
            &label,
            tick_scale(l.yb),
            "end",
        );
    }
    drawing.line(LEFT, TOP, LEFT, BOTTOM, BLACK);
    drawing.line(LEFT, BOTTOM, RIGHT, BOTTOM, BLACK);
    if input.kind == "histogram" {
        for (i, count) in l.counts.iter().enumerate() {
            let a = position(l.edges[i], l.xb, LEFT, RIGHT).round() as usize;
            let b = position(l.edges[i + 1], l.xb, LEFT, RIGHT).round() as usize;
            let y = BOTTOM - position(*count as f64, l.yb, 0, BOTTOM - TOP).round() as usize;
            drawing.rect(a, y, b, BOTTOM, BLUE);
        }
    } else {
        let mut previous = None;
        for (x, y) in l.x.iter().zip(&l.y) {
            let point = (
                position(*x, l.xb, LEFT, RIGHT).round() as usize,
                BOTTOM - position(*y, l.yb, 0, BOTTOM - TOP).round() as usize,
            );
            if input.kind == "line"
                && let Some((px, py)) = previous
            {
                drawing.line(px, py, point.0, point.1, BLUE);
            }
            drawing.dot(point.0, point.1);
            previous = Some(point);
        }
    }
    drawing.finish()
}
struct Drawing {
    canvas: Option<Canvas>,
    svg: String,
}
impl Drawing {
    fn new(png: bool) -> Self {
        Self {
            canvas: png.then(|| Canvas::new(WIDTH, HEIGHT)),
            svg: format!(
                "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{WIDTH}\" height=\"{HEIGHT}\" viewBox=\"0 0 {WIDTH} {HEIGHT}\"><rect width=\"100%\" height=\"100%\" fill=\"white\"/>\n"
            ),
        }
    }
    fn line(&mut self, a: usize, b: usize, c: usize, d: usize, colour: [u8; 3]) {
        if let Some(canvas) = &mut self.canvas {
            canvas.line(a as isize, b as isize, c as isize, d as isize, colour);
        } else {
            self.svg.push_str(&format!(
                "<line x1=\"{a}\" y1=\"{b}\" x2=\"{c}\" y2=\"{d}\" stroke=\"{}\"/>\n",
                css(colour)
            ));
        }
    }
    fn rect(&mut self, a: usize, b: usize, c: usize, d: usize, colour: [u8; 3]) {
        if let Some(canvas) = &mut self.canvas {
            canvas.fill_rect(a, b, c, d, colour);
        } else {
            self.svg.push_str(&format!(
                "<rect x=\"{a}\" y=\"{b}\" width=\"{}\" height=\"{}\" fill=\"{}\"/>\n",
                c - a,
                d - b,
                css(colour)
            ));
        }
    }
    fn dot(&mut self, x: usize, y: usize) {
        if let Some(canvas) = &mut self.canvas {
            canvas.dot(x as isize, y as isize, BLUE);
        } else {
            self.svg.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"3\" height=\"3\" fill=\"#2469af\"/>\n",
                x - 1,
                y - 1
            ));
        }
    }
    fn text(&mut self, x: usize, y: usize, text: &str, scale: usize, anchor: &str) {
        if let Some(canvas) = &mut self.canvas {
            let width = text.chars().count() * 6 * scale;
            let offset = match anchor {
                "middle" => width / 2,
                "end" => width,
                _ => 0,
            };
            let mut cursor = x.saturating_sub(offset);
            for ch in text.chars() {
                for (row, bits) in letter(ch).iter().enumerate() {
                    for col in 0..5 {
                        if bits & (1 << (4 - col)) != 0 {
                            canvas.fill_rect(
                                cursor + col * scale,
                                y + row * scale,
                                cursor + (col + 1) * scale,
                                y + (row + 1) * scale,
                                BLACK,
                            );
                        }
                    }
                }
                cursor += 6 * scale;
            }
        } else {
            self.svg.push_str(&format!("<text x=\"{x}\" y=\"{}\" font-family=\"monospace\" font-size=\"{}\" text-anchor=\"{anchor}\" fill=\"#232830\">{}</text>\n",y+7*scale,9*scale,crate::output::xml(text)));
        }
    }
    fn finish(self) -> Vec<u8> {
        if let Some(canvas) = self.canvas {
            crate::output::encode_png(WIDTH, HEIGHT, &canvas.pixels)
        } else {
            format!("{}</svg>\n", self.svg).into_bytes()
        }
    }
}
fn css(c: [u8; 3]) -> String {
    format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2])
}

fn supported(c: char) -> bool {
    c.is_ascii_alphanumeric() || " .,-+_/:()=[]^*;!?&<>%\"'\\|{}".contains(c)
}
// Case is scientifically significant: never uppercase units or user labels.
fn letter(c: char) -> [u8; 7] {
    match c {
        'a' => [0, 0, 14, 1, 15, 17, 15],
        'b' => [16, 16, 30, 17, 17, 17, 30],
        'c' => [0, 0, 14, 17, 16, 17, 14],
        'd' => [1, 1, 15, 17, 17, 17, 15],
        'e' => [0, 0, 14, 17, 31, 16, 14],
        'f' => [6, 8, 8, 28, 8, 8, 8],
        'g' => [0, 15, 17, 17, 15, 1, 14],
        'h' => [16, 16, 30, 17, 17, 17, 17],
        'i' => [4, 0, 12, 4, 4, 4, 14],
        'j' => [2, 0, 6, 2, 2, 18, 12],
        'k' => [16, 16, 18, 20, 24, 20, 18],
        'l' => [12, 4, 4, 4, 4, 4, 14],
        'm' => [0, 0, 26, 21, 21, 21, 21],
        'n' => [0, 0, 30, 17, 17, 17, 17],
        'o' => [0, 0, 14, 17, 17, 17, 14],
        'p' => [0, 30, 17, 17, 30, 16, 16],
        'q' => [0, 15, 17, 17, 15, 1, 1],
        'r' => [0, 0, 22, 25, 16, 16, 16],
        's' => [0, 0, 15, 16, 14, 1, 30],
        't' => [8, 8, 28, 8, 8, 9, 6],
        'u' => [0, 0, 17, 17, 17, 19, 13],
        'v' => [0, 0, 17, 17, 17, 10, 4],
        'w' => [0, 0, 17, 17, 21, 21, 10],
        'x' => [0, 0, 17, 10, 4, 10, 17],
        'y' => [0, 17, 17, 17, 15, 1, 14],
        'z' => [0, 0, 31, 2, 4, 8, 31],
        '[' => [14, 8, 8, 8, 8, 8, 14],
        ']' => [14, 2, 2, 2, 2, 2, 14],
        '^' => [4, 10, 17, 0, 0, 0, 0],
        '*' => [0, 21, 14, 31, 14, 21, 0],
        ';' => [0, 4, 4, 0, 4, 4, 8],
        '!' => [4, 4, 4, 4, 4, 0, 4],
        '?' => [14, 17, 1, 2, 4, 0, 4],
        '&' => [12, 18, 20, 8, 21, 18, 13],
        '<' => [2, 4, 8, 16, 8, 4, 2],
        '>' => [8, 4, 2, 1, 2, 4, 8],
        '%' => [25, 26, 2, 4, 8, 11, 19],
        '"' => [10, 10, 10, 0, 0, 0, 0],
        '\'' => [4, 4, 8, 0, 0, 0, 0],
        '\\' => [16, 8, 8, 4, 2, 2, 1],
        '|' => [4, 4, 4, 4, 4, 4, 4],
        '{' => [2, 4, 4, 8, 4, 4, 2],
        '}' => [8, 4, 4, 2, 4, 4, 8],
        c => crate::output::glyph(c),
    }
}

#[cfg(test)]
mod provenance_compatibility_tests {
    use super::*;
    #[test]
    fn historical_v1_pairs_reconstruct_without_inventing_provenance() {
        for extension in ["png", "svg"] {
            let axis = Axis {
                label: "x".into(),
                dimension: crate::quantity::DIMENSIONLESS,
                values_bits: [0_f64, 1.].map(bits).to_vec(),
            };
            let input = Input {
                figure: format!("a.{extension}"),
                kind: "line".into(),
                title: "Old data".into(),
                x: axis.clone(),
                y: Some(axis),
                bins: None,
            };
            let outputs = build_with_policy(&input, LEGACY_POLICY).unwrap();
            let document: serde_json::Value = serde_json::from_slice(&outputs[1].bytes).unwrap();
            assert!(document.get("manifest_sha256").is_none());
            assert!(
                crate::plot_provenance::extract(&outputs[0].bytes, extension == "png").is_err()
            );
            verify_pair(
                &outputs[0].bytes,
                &outputs[1].bytes,
                &crate::output::artifact_descriptor(&outputs[0]),
                &crate::output::artifact_descriptor(&outputs[1]),
            )
            .unwrap();
            assert_ne!(outputs[0].bytes, build(&input).unwrap()[0].bytes);
        }
    }
}
