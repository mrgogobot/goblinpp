use crate::error::{GoblinError, Result};
use serde_json::{Value, json};
use std::path::{Component, Path};

pub const MAX_OUTPUT_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_PLOT_POINTS: usize = 200_000;
const WIDTH: usize = 960;
const HEIGHT: usize = 600;

#[derive(Debug, Clone)]
pub struct GeneratedOutput {
    pub name: String,
    pub media_type: String,
    pub producer: String,
    pub bytes: Vec<u8>,
    pub metadata: Value,
}

impl GeneratedOutput {
    pub fn new(
        name: &str,
        media_type: &str,
        producer: &str,
        bytes: Vec<u8>,
        metadata: Value,
    ) -> Result<Self> {
        validate_name(name)?;
        if bytes.len() > MAX_OUTPUT_BYTES {
            return Err(GoblinError::artifact(format!(
                "Generated output {name:?} is {} bytes; the per-file limit is {MAX_OUTPUT_BYTES} bytes.",
                bytes.len()
            )));
        }
        Ok(Self {
            name: name.into(),
            media_type: media_type.into(),
            producer: producer.into(),
            bytes,
            metadata,
        })
    }
}

pub fn validate_name(name: &str) -> Result<()> {
    if name.is_empty() || name == "." || name == ".." || name.chars().any(char::is_control) {
        return Err(GoblinError::artifact(
            "Generated output name must be a non-empty ordinary filename without control characters.",
        ));
    }
    let mut components = Path::new(name).components();
    if !matches!(components.next(), Some(Component::Normal(_))) || components.next().is_some() {
        return Err(GoblinError::artifact(format!(
            "Generated output {name:?} must be a filename, not an absolute path or directory traversal. Outputs live inside RUN_DIR/outputs."
        )));
    }
    Ok(())
}

pub fn extension(name: &str) -> Result<String> {
    validate_name(name)?;
    Path::new(name)
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .ok_or_else(|| {
            GoblinError::artifact(format!("Generated output {name:?} needs an extension."))
        })
}

pub fn delimited_bytes(rows: &[Vec<String>], delimiter: u8) -> Vec<u8> {
    let mut output = Vec::new();
    for row in rows {
        for (index, cell) in row.iter().enumerate() {
            if index != 0 {
                output.push(delimiter);
            }
            if delimiter == b',' {
                csv_cell(cell, &mut output);
            } else {
                let cleaned = cell.replace(['\t', '\r', '\n'], " ");
                output.extend_from_slice(cleaned.as_bytes());
            }
        }
        output.push(b'\n');
    }
    output
}

fn csv_cell(cell: &str, output: &mut Vec<u8>) {
    if cell
        .bytes()
        .any(|byte| matches!(byte, b',' | b'"' | b'\r' | b'\n'))
    {
        output.push(b'"');
        for byte in cell.bytes() {
            if byte == b'"' {
                output.push(b'"');
            }
            output.push(byte);
        }
        output.push(b'"');
    } else {
        output.extend_from_slice(cell.as_bytes());
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Sampling {
    pub population_rows: usize,
    pub requested_points: usize,
    pub examined_rows: usize,
    pub valid_points: usize,
}

impl Sampling {
    pub fn json(self) -> Value {
        json!({
            "method": "deterministic-even-row-sample",
            "population_rows": self.population_rows,
            "requested_points": self.requested_points,
            "examined_rows": self.examined_rows,
            "valid_points": self.valid_points,
        })
    }
}

pub fn histogram(
    name: &str,
    title: &str,
    x_label: &str,
    values: &[f64],
    bins: usize,
    sampling: Sampling,
    input_sha256: &str,
) -> Result<GeneratedOutput> {
    if !(2..=500).contains(&bins) {
        return Err(GoblinError::artifact(
            "Histogram bin count must be between 2 and 500.",
        ));
    }
    if values.is_empty() {
        return Err(GoblinError::data(
            "Histogram sample contains no finite, non-null values.",
        ));
    }
    let minimum = values.iter().copied().fold(f64::INFINITY, f64::min);
    let maximum = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let mut counts = vec![0_usize; bins];
    if minimum == maximum {
        counts[bins / 2] = values.len();
    } else {
        for value in values {
            let normalized = (*value - minimum) / (maximum - minimum);
            let index = ((normalized * bins as f64).floor() as usize).min(bins - 1);
            counts[index] += 1;
        }
    }
    let metadata = json!({
        "kind": "histogram",
        "input_sha256": input_sha256,
        "x_label": x_label,
        "bins": bins,
        "sample_minimum": minimum,
        "sample_maximum": maximum,
        "sampling": sampling.json(),
    });
    let bytes = match extension(name)?.as_str() {
        "svg" => histogram_svg(title, x_label, &counts, minimum, maximum).into_bytes(),
        "png" => histogram_png(title, x_label, &counts, minimum, maximum),
        other => {
            return Err(GoblinError::artifact(format!(
                "Histogram output extension .{other} is unsupported; use .svg or .png."
            )));
        }
    };
    let media_type = if name.to_ascii_lowercase().ends_with(".svg") {
        "image/svg+xml"
    } else {
        "image/png"
    };
    GeneratedOutput::new(name, media_type, "plot_fits_histogram", bytes, metadata)
}

pub struct ScatterRequest<'a> {
    pub name: &'a str,
    pub title: &'a str,
    pub x_label: &'a str,
    pub y_label: &'a str,
    pub points: &'a [(f64, f64)],
    pub sampling: Sampling,
    pub input_sha256: &'a str,
}

pub fn scatter(request: ScatterRequest<'_>) -> Result<GeneratedOutput> {
    if request.points.is_empty() {
        return Err(GoblinError::data(
            "Scatter sample contains no rows with two finite, non-null values.",
        ));
    }
    let (x_min, x_max, y_min, y_max) = scatter_bounds(request.points);
    let metadata = json!({
        "kind": "scatter",
        "input_sha256": request.input_sha256,
        "x_label": request.x_label,
        "y_label": request.y_label,
        "sample_x_minimum": x_min,
        "sample_x_maximum": x_max,
        "sample_y_minimum": y_min,
        "sample_y_maximum": y_max,
        "sampling": request.sampling.json(),
    });
    let bytes = match extension(request.name)?.as_str() {
        "svg" => scatter_svg(
            request.title,
            request.x_label,
            request.y_label,
            request.points,
            (x_min, x_max, y_min, y_max),
        )
        .into_bytes(),
        "png" => scatter_png(
            request.title,
            request.x_label,
            request.y_label,
            request.points,
            (x_min, x_max, y_min, y_max),
        ),
        other => {
            return Err(GoblinError::artifact(format!(
                "Scatter output extension .{other} is unsupported; use .svg or .png."
            )));
        }
    };
    let media_type = if request.name.to_ascii_lowercase().ends_with(".svg") {
        "image/svg+xml"
    } else {
        "image/png"
    };
    GeneratedOutput::new(
        request.name,
        media_type,
        "plot_fits_scatter",
        bytes,
        metadata,
    )
}

fn histogram_svg(title: &str, label: &str, counts: &[usize], min: f64, max: f64) -> String {
    let plot_x = 90.0;
    let plot_y = 70.0;
    let plot_w = 820.0;
    let plot_h = 440.0;
    let peak = counts.iter().copied().max().unwrap_or(1).max(1) as f64;
    let bar_w = plot_w / counts.len() as f64;
    let mut bars = String::new();
    for (index, count) in counts.iter().enumerate() {
        let height = *count as f64 / peak * plot_h;
        bars.push_str(&format!(
            "<rect x=\"{:.3}\" y=\"{:.3}\" width=\"{:.3}\" height=\"{:.3}\" fill=\"#31a354\" stroke=\"#176c36\" stroke-width=\"0.5\"/>\n",
            plot_x + index as f64 * bar_w,
            plot_y + plot_h - height,
            bar_w.max(1.0),
            height
        ));
    }
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{WIDTH}\" height=\"{HEIGHT}\" viewBox=\"0 0 {WIDTH} {HEIGHT}\">\n<rect width=\"100%\" height=\"100%\" fill=\"white\"/>\n<text x=\"480\" y=\"35\" text-anchor=\"middle\" font-family=\"sans-serif\" font-size=\"22\">{}</text>\n<line x1=\"90\" y1=\"510\" x2=\"910\" y2=\"510\" stroke=\"#222\"/>\n<line x1=\"90\" y1=\"70\" x2=\"90\" y2=\"510\" stroke=\"#222\"/>\n{}<text x=\"500\" y=\"570\" text-anchor=\"middle\" font-family=\"sans-serif\" font-size=\"16\">{}</text>\n<text x=\"90\" y=\"535\" font-family=\"monospace\" font-size=\"13\">{}</text>\n<text x=\"910\" y=\"535\" text-anchor=\"end\" font-family=\"monospace\" font-size=\"13\">{}</text>\n<text x=\"18\" y=\"290\" transform=\"rotate(-90 18 290)\" text-anchor=\"middle\" font-family=\"sans-serif\" font-size=\"16\">sample count</text>\n</svg>\n",
        xml(title),
        bars,
        xml(label),
        number(min),
        number(max)
    )
}

fn scatter_svg(
    title: &str,
    x_label: &str,
    y_label: &str,
    points: &[(f64, f64)],
    bounds: (f64, f64, f64, f64),
) -> String {
    let (x_min, x_max, y_min, y_max) = expanded_bounds(bounds);
    let mut circles = String::new();
    for (x, y) in points {
        let px = 90.0 + (*x - x_min) / (x_max - x_min) * 820.0;
        let py = 510.0 - (*y - y_min) / (y_max - y_min) * 440.0;
        circles.push_str(&format!(
            "<circle cx=\"{px:.3}\" cy=\"{py:.3}\" r=\"1.6\" fill=\"#2171b5\" fill-opacity=\"0.45\"/>\n"
        ));
    }
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{WIDTH}\" height=\"{HEIGHT}\" viewBox=\"0 0 {WIDTH} {HEIGHT}\">\n<rect width=\"100%\" height=\"100%\" fill=\"white\"/>\n<text x=\"480\" y=\"35\" text-anchor=\"middle\" font-family=\"sans-serif\" font-size=\"22\">{}</text>\n<line x1=\"90\" y1=\"510\" x2=\"910\" y2=\"510\" stroke=\"#222\"/>\n<line x1=\"90\" y1=\"70\" x2=\"90\" y2=\"510\" stroke=\"#222\"/>\n{}<text x=\"500\" y=\"570\" text-anchor=\"middle\" font-family=\"sans-serif\" font-size=\"16\">{}</text>\n<text x=\"18\" y=\"290\" transform=\"rotate(-90 18 290)\" text-anchor=\"middle\" font-family=\"sans-serif\" font-size=\"16\">{}</text>\n<text x=\"90\" y=\"535\" font-family=\"monospace\" font-size=\"13\">{}</text>\n<text x=\"910\" y=\"535\" text-anchor=\"end\" font-family=\"monospace\" font-size=\"13\">{}</text>\n<text x=\"82\" y=\"80\" text-anchor=\"end\" font-family=\"monospace\" font-size=\"13\">{}</text>\n<text x=\"82\" y=\"510\" text-anchor=\"end\" font-family=\"monospace\" font-size=\"13\">{}</text>\n</svg>\n",
        xml(title),
        circles,
        xml(x_label),
        xml(y_label),
        number(x_min),
        number(x_max),
        number(y_max),
        number(y_min)
    )
}

fn histogram_png(title: &str, label: &str, counts: &[usize], min: f64, max: f64) -> Vec<u8> {
    let mut canvas = Canvas::new(WIDTH, HEIGHT);
    canvas.line(90, 510, 910, 510, [30, 30, 30]);
    canvas.line(90, 70, 90, 510, [30, 30, 30]);
    let peak = counts.iter().copied().max().unwrap_or(1).max(1);
    for (index, count) in counts.iter().enumerate() {
        let left = 90 + index * 820 / counts.len();
        let right = 90 + (index + 1) * 820 / counts.len();
        let height = count * 440 / peak;
        canvas.fill_rect(left, 510 - height, right.max(left + 1), 510, [49, 163, 84]);
    }
    canvas.text_center(480, 24, title, 3, [20, 20, 20]);
    canvas.text_center(500, 552, label, 2, [20, 20, 20]);
    canvas.text(90, 525, &number(min), 1, [20, 20, 20]);
    canvas.text_right(910, 525, &number(max), 1, [20, 20, 20]);
    canvas.text(10, 280, "SAMPLE COUNT", 1, [20, 20, 20]);
    encode_png(WIDTH, HEIGHT, &canvas.pixels)
}

fn scatter_png(
    title: &str,
    x_label: &str,
    y_label: &str,
    points: &[(f64, f64)],
    bounds: (f64, f64, f64, f64),
) -> Vec<u8> {
    let (x_min, x_max, y_min, y_max) = expanded_bounds(bounds);
    let mut canvas = Canvas::new(WIDTH, HEIGHT);
    canvas.line(90, 510, 910, 510, [30, 30, 30]);
    canvas.line(90, 70, 90, 510, [30, 30, 30]);
    for (x, y) in points {
        let px = 90.0 + (*x - x_min) / (x_max - x_min) * 820.0;
        let py = 510.0 - (*y - y_min) / (y_max - y_min) * 440.0;
        canvas.dot(px.round() as isize, py.round() as isize, [33, 113, 181]);
    }
    canvas.text_center(480, 24, title, 3, [20, 20, 20]);
    canvas.text_center(500, 552, x_label, 2, [20, 20, 20]);
    canvas.text(10, 280, y_label, 1, [20, 20, 20]);
    canvas.text(90, 525, &number(x_min), 1, [20, 20, 20]);
    canvas.text_right(910, 525, &number(x_max), 1, [20, 20, 20]);
    canvas.text_right(82, 75, &number(y_max), 1, [20, 20, 20]);
    canvas.text_right(82, 500, &number(y_min), 1, [20, 20, 20]);
    encode_png(WIDTH, HEIGHT, &canvas.pixels)
}

fn scatter_bounds(points: &[(f64, f64)]) -> (f64, f64, f64, f64) {
    points.iter().fold(
        (
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
        ),
        |(x_min, x_max, y_min, y_max), (x, y)| {
            (x_min.min(*x), x_max.max(*x), y_min.min(*y), y_max.max(*y))
        },
    )
}

fn expanded_bounds(
    (mut x_min, mut x_max, mut y_min, mut y_max): (f64, f64, f64, f64),
) -> (f64, f64, f64, f64) {
    if x_min == x_max {
        let pad = x_min.abs().max(1.0) * 0.05;
        x_min -= pad;
        x_max += pad;
    }
    if y_min == y_max {
        let pad = y_min.abs().max(1.0) * 0.05;
        y_min -= pad;
        y_max += pad;
    }
    (x_min, x_max, y_min, y_max)
}

fn xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn number(value: f64) -> String {
    if value == 0.0 {
        return "0".into();
    }
    if value.abs() >= 1e6 || value.abs() < 1e-3 {
        format!("{value:.4e}")
    } else {
        format!("{value:.6}")
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}

struct Canvas {
    width: usize,
    height: usize,
    pixels: Vec<u8>,
}

impl Canvas {
    fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            pixels: vec![255; width * height * 3],
        }
    }

    fn set(&mut self, x: isize, y: isize, colour: [u8; 3]) {
        if x < 0 || y < 0 || x >= self.width as isize || y >= self.height as isize {
            return;
        }
        let offset = (y as usize * self.width + x as usize) * 3;
        self.pixels[offset..offset + 3].copy_from_slice(&colour);
    }

    fn line(&mut self, mut x0: isize, mut y0: isize, x1: isize, y1: isize, colour: [u8; 3]) {
        let dx = (x1 - x0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let dy = -(y1 - y0).abs();
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut error = dx + dy;
        loop {
            self.set(x0, y0, colour);
            if x0 == x1 && y0 == y1 {
                break;
            }
            let twice = 2 * error;
            if twice >= dy {
                error += dy;
                x0 += sx;
            }
            if twice <= dx {
                error += dx;
                y0 += sy;
            }
        }
    }

    fn fill_rect(&mut self, left: usize, top: usize, right: usize, bottom: usize, colour: [u8; 3]) {
        for y in top.min(self.height)..bottom.min(self.height) {
            for x in left.min(self.width)..right.min(self.width) {
                self.set(x as isize, y as isize, colour);
            }
        }
    }

    fn dot(&mut self, x: isize, y: isize, colour: [u8; 3]) {
        for dy in -1..=1 {
            for dx in -1..=1 {
                self.set(x + dx, y + dy, colour);
            }
        }
    }

    fn text(&mut self, x: usize, y: usize, value: &str, scale: usize, colour: [u8; 3]) {
        let mut cursor = x;
        for ch in value.to_ascii_uppercase().chars() {
            let glyph = glyph(ch);
            for (row, bits) in glyph.iter().enumerate() {
                for column in 0..5 {
                    if bits & (1 << (4 - column)) != 0 {
                        self.fill_rect(
                            cursor + column * scale,
                            y + row * scale,
                            cursor + (column + 1) * scale,
                            y + (row + 1) * scale,
                            colour,
                        );
                    }
                }
            }
            cursor += 6 * scale;
        }
    }

    fn text_center(&mut self, x: usize, y: usize, value: &str, scale: usize, colour: [u8; 3]) {
        let width = value.chars().count() * 6 * scale;
        self.text(x.saturating_sub(width / 2), y, value, scale, colour);
    }

    fn text_right(&mut self, x: usize, y: usize, value: &str, scale: usize, colour: [u8; 3]) {
        let width = value.chars().count() * 6 * scale;
        self.text(x.saturating_sub(width), y, value, scale, colour);
    }
}

fn glyph(ch: char) -> [u8; 7] {
    match ch {
        'A' => [14, 17, 17, 31, 17, 17, 17],
        'B' => [30, 17, 17, 30, 17, 17, 30],
        'C' => [14, 17, 16, 16, 16, 17, 14],
        'D' => [30, 17, 17, 17, 17, 17, 30],
        'E' => [31, 16, 16, 30, 16, 16, 31],
        'F' => [31, 16, 16, 30, 16, 16, 16],
        'G' => [14, 17, 16, 23, 17, 17, 15],
        'H' => [17, 17, 17, 31, 17, 17, 17],
        'I' => [14, 4, 4, 4, 4, 4, 14],
        'J' => [7, 2, 2, 2, 18, 18, 12],
        'K' => [17, 18, 20, 24, 20, 18, 17],
        'L' => [16, 16, 16, 16, 16, 16, 31],
        'M' => [17, 27, 21, 21, 17, 17, 17],
        'N' => [17, 25, 21, 19, 17, 17, 17],
        'O' => [14, 17, 17, 17, 17, 17, 14],
        'P' => [30, 17, 17, 30, 16, 16, 16],
        'Q' => [14, 17, 17, 17, 21, 18, 13],
        'R' => [30, 17, 17, 30, 20, 18, 17],
        'S' => [15, 16, 16, 14, 1, 1, 30],
        'T' => [31, 4, 4, 4, 4, 4, 4],
        'U' => [17, 17, 17, 17, 17, 17, 14],
        'V' => [17, 17, 17, 17, 17, 10, 4],
        'W' => [17, 17, 17, 21, 21, 21, 10],
        'X' => [17, 17, 10, 4, 10, 17, 17],
        'Y' => [17, 17, 10, 4, 4, 4, 4],
        'Z' => [31, 1, 2, 4, 8, 16, 31],
        '0' => [14, 17, 19, 21, 25, 17, 14],
        '1' => [4, 12, 4, 4, 4, 4, 14],
        '2' => [14, 17, 1, 2, 4, 8, 31],
        '3' => [30, 1, 1, 14, 1, 1, 30],
        '4' => [2, 6, 10, 18, 31, 2, 2],
        '5' => [31, 16, 16, 30, 1, 1, 30],
        '6' => [14, 16, 16, 30, 17, 17, 14],
        '7' => [31, 1, 2, 4, 8, 8, 8],
        '8' => [14, 17, 17, 14, 17, 17, 14],
        '9' => [14, 17, 17, 15, 1, 1, 14],
        '.' => [0, 0, 0, 0, 0, 12, 12],
        ',' => [0, 0, 0, 0, 0, 12, 8],
        '-' => [0, 0, 0, 31, 0, 0, 0],
        '+' => [0, 4, 4, 31, 4, 4, 0],
        '_' => [0, 0, 0, 0, 0, 0, 31],
        '/' => [1, 2, 4, 8, 16, 0, 0],
        ':' => [0, 12, 12, 0, 12, 12, 0],
        '(' => [2, 4, 8, 8, 8, 4, 2],
        ')' => [8, 4, 2, 2, 2, 4, 8],
        '=' => [0, 31, 0, 31, 0, 0, 0],
        ' ' => [0; 7],
        _ => [31, 17, 2, 4, 4, 0, 4],
    }
}

fn encode_png(width: usize, height: usize, rgb: &[u8]) -> Vec<u8> {
    let mut filtered = Vec::with_capacity((width * 3 + 1) * height);
    for row in rgb.chunks_exact(width * 3) {
        filtered.push(0);
        filtered.extend_from_slice(row);
    }
    let compressed = zlib_store(&filtered);
    let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&(width as u32).to_be_bytes());
    ihdr.extend_from_slice(&(height as u32).to_be_bytes());
    ihdr.extend_from_slice(&[8, 2, 0, 0, 0]);
    png_chunk(&mut png, b"IHDR", &ihdr);
    png_chunk(&mut png, b"IDAT", &compressed);
    png_chunk(&mut png, b"IEND", &[]);
    png
}

fn zlib_store(bytes: &[u8]) -> Vec<u8> {
    let mut output = vec![0x78, 0x01];
    let mut cursor = 0;
    while cursor < bytes.len() {
        let length = (bytes.len() - cursor).min(u16::MAX as usize);
        let final_block = cursor + length == bytes.len();
        output.push(u8::from(final_block));
        let len = length as u16;
        output.extend_from_slice(&len.to_le_bytes());
        output.extend_from_slice(&(!len).to_le_bytes());
        output.extend_from_slice(&bytes[cursor..cursor + length]);
        cursor += length;
    }
    output.extend_from_slice(&adler32(bytes).to_be_bytes());
    output
}

fn png_chunk(output: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    output.extend_from_slice(&(data.len() as u32).to_be_bytes());
    output.extend_from_slice(kind);
    output.extend_from_slice(data);
    let mut crc_input = Vec::with_capacity(4 + data.len());
    crc_input.extend_from_slice(kind);
    crc_input.extend_from_slice(data);
    output.extend_from_slice(&crc32(&crc_input).to_be_bytes());
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = u32::MAX;
    for byte in bytes {
        crc ^= *byte as u32;
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xedb8_8320 & (0_u32.wrapping_sub(crc & 1)));
        }
    }
    !crc
}

fn adler32(bytes: &[u8]) -> u32 {
    let mut a = 1_u32;
    let mut b = 0_u32;
    for byte in bytes {
        a = (a + *byte as u32) % 65_521;
        b = (b + a) % 65_521;
    }
    (b << 16) | a
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_escaping_is_rfc4180_style() {
        let rows = vec![vec!["a,b".into(), "say \"hi\"".into()]];
        assert_eq!(
            delimited_bytes(&rows, b','),
            b"\"a,b\",\"say \"\"hi\"\"\"\n"
        );
    }

    #[test]
    fn output_names_cannot_escape_run_directory() {
        assert!(validate_name("plot.png").is_ok());
        assert!(validate_name("../plot.png").is_err());
        assert!(validate_name("nested/plot.png").is_err());
        assert!(validate_name("/tmp/plot.png").is_err());
    }

    #[test]
    fn png_encoder_has_valid_signature_and_chunks() {
        let pixels = vec![255; 2 * 2 * 3];
        let png = encode_png(2, 2, &pixels);
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
        assert!(png.windows(4).any(|value| value == b"IHDR"));
        assert!(png.windows(4).any(|value| value == b"IDAT"));
        assert!(png.ends_with(&[0xae, 0x42, 0x60, 0x82]));
    }
}
