use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::Path;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
fn require(ok: bool, why: &str) -> Result<()> {
    if ok { Ok(()) } else { Err(why.into()) }
}
fn hash(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
fn create(p: &Path) -> Result<File> {
    Ok(OpenOptions::new().write(true).create_new(true).open(p)?)
}
fn word(b: &[u8], record: usize, field: usize) -> u32 {
    let i = record * 20 + field * 4;
    u32::from_le_bytes(b[i..i + 4].try_into().unwrap())
}
fn sample(b: &[u8], record: usize) -> Option<f64> {
    let s = f32::from_bits(word(b, record, 0));
    let w = f32::from_bits(word(b, record, 3));
    let d = word(b, record, 2);
    if s.is_finite() && d == 0 && w.is_finite() && w > 0. {
        Some(s as f64)
    } else {
        None
    }
}
fn median(v: &mut [f64]) -> f64 {
    v.sort_by(f64::total_cmp);
    let n = v.len();
    if n % 2 == 1 {
        v[n / 2]
    } else {
        (v[n / 2 - 1] + v[n / 2]) / 2.
    }
}
fn compensated_sum(v: impl Iterator<Item = f64>) -> f64 {
    let mut total = 0.;
    let mut correction = 0.;
    for item in v {
        let y = item - correction;
        let t = total + y;
        correction = (t - total) - y;
        total = t;
    }
    total
}
fn finite_text(v: Option<f64>) -> String {
    v.filter(|x| x.is_finite())
        .map(|x| x.to_string())
        .unwrap_or_default()
}
fn geometry(
    nx: usize,
    ny: usize,
    x: f64,
    y: f64,
    r: f64,
    ring: [f64; 2],
) -> (Vec<usize>, Vec<usize>) {
    let mut ap = Vec::new();
    let mut sky = Vec::new();
    for yy in 0..ny {
        for xx in 0..nx {
            let d = (xx as f64 - x).hypot(yy as f64 - y);
            let index = yy * nx + xx;
            if d < r {
                ap.push(index)
            }
            if d >= ring[0] && d < ring[1] {
                sky.push(index)
            }
        }
    }
    (ap, sky)
}
fn checked_shape(dim: &Value) -> Result<(usize, usize, usize, usize)> {
    let dim = dim.as_array().ok_or("Missing shape")?;
    require(dim.len() == 3, "Expected three dimensions")?;
    let axis = |i: usize| -> Result<usize> {
        Ok(usize::try_from(dim[i].as_u64().ok_or("Invalid axis")?)?)
    };
    let (nx, ny, nz) = (axis(0)?, axis(1)?, axis(2)?);
    let count = nx.checked_mul(ny).and_then(|v| v.checked_mul(nz));
    require(
        nx > 0 && ny > 0 && nz > 0 && count.is_some_and(|v| v <= 10_000_000),
        "Unsupported resource size",
    )?;
    Ok((nx, ny, nz, count.ok_or("Shape overflow")? * 20))
}
fn main() {
    if let Err(e) = run() {
        eprintln!("REGIONAL_AUDIT_ERROR: {e}");
        std::process::exit(1)
    }
}
fn run() -> Result<()> {
    let a: Vec<_> = std::env::args().collect();
    require(
        a.len() == 5,
        "Usage: regional-audit EXTRACTED_RESULTS PROTOCOL EXPECTED_PROTOCOL_SHA256 NEW_OUTPUT_DIR",
    )?;
    let input = Path::new(&a[1]);
    let proto_bytes = fs::read(&a[2])?;
    require(
        hash(&proto_bytes) == a[3],
        "Protocol external hash mismatch",
    )?;
    let p: Value = serde_json::from_slice(&proto_bytes)?;
    require(
        p["schema"] == "goblin.study.matched-apertures.v1",
        "Unsupported protocol",
    )?;
    require(
        p["mask_policy"] == "documented_convenience",
        "Unsupported mask",
    )?;
    let receipt_bytes = fs::read(input.join("receipt.json"))?;
    require(
        hash(&receipt_bytes)
            == p["input_receipt_sha256"]
                .as_str()
                .ok_or("Missing input anchor")?,
        "Extraction receipt hash mismatch",
    )?;
    let r: Value = serde_json::from_slice(&receipt_bytes)?;
    let (nx, ny, nz, byte_count) = checked_shape(&r["metadata"]["dimensions"])?;
    let binary_path = input.join("spectra.bin");
    require(
        fs::metadata(&binary_path)?.len() == byte_count as u64,
        "Binary byte count outside pinned shape",
    )?;
    let b = fs::read(binary_path)?;
    require(
        hash(&b)
            == r["outputs"]["spectra.bin"]["sha256"]
                .as_str()
                .ok_or("Missing binary hash")?,
        "Binary hash mismatch",
    )?;
    require(b.len() == byte_count, "Binary length mismatch")?;
    let cd1 = p["cdelt1_deg"].as_f64().ok_or("cdelt1")?;
    let cd2 = p["cdelt2_deg"].as_f64().ok_or("cdelt2")?;
    let conversion = cd1.abs().to_radians() * cd2.abs().to_radians() * 1e15;
    require(
        conversion.is_finite() && conversion > 0.,
        "Invalid conversion",
    )?;
    let ring = [
        p["background_ring_pixels"][0].as_f64().ok_or("ring")?,
        p["background_ring_pixels"][1].as_f64().ok_or("ring")?,
    ];
    require(ring[0] >= 0. && ring[1] > ring[0], "Bad ring")?;
    require(
        p["minimum_aperture_fraction"] == 0.8
            && p["minimum_background_rule"] == "n_sky >= min(20,0.5*n_geometric_sky)",
        "Unsupported coverage thresholds",
    )?;
    let aps = p["apertures"].as_array().ok_or("Missing apertures")?;
    require(
        !aps.is_empty() && aps.len() <= 16,
        "Aperture resource limit",
    )?;
    let out = Path::new(&a[4]);
    fs::create_dir(out)?;
    let mut table = BufWriter::new(create(&out.join("rust_matched_apertures.csv"))?);
    write!(table, "channel0,wavelength_um")?;
    let mut geometry_list = Vec::new();
    let mut descriptions = Vec::new();
    for ap in aps {
        let name = ap["name"].as_str().ok_or("name")?;
        require(
            name.bytes()
                .all(|x| x.is_ascii_alphanumeric() || x == b'_' || x == b'.'),
            "Unsafe name",
        )?;
        let x = ap["x0"].as_f64().ok_or("x")?;
        let y = ap["y0"].as_f64().ok_or("y")?;
        let radius = ap["radius_pixels"].as_f64().ok_or("radius")?;
        require(
            x.is_finite() && y.is_finite() && radius.is_finite() && radius > 0. && radius <= 128.,
            "Bad geometry",
        )?;
        let (selected, sky) = geometry(nx, ny, x, y, radius, ring);
        require(!selected.is_empty() && !sky.is_empty(), "Empty geometry")?;
        descriptions.push(json!({"name":name,"geometric_aperture_spaxels":selected.len(),"geometric_sky_spaxels":sky.len()}));
        geometry_list.push((selected, sky));
        write!(
            table,
            ",{name}_flux_njy,{name}_background_mjy_sr,{name}_valid_aperture_spaxels,{name}_valid_sky_spaxels"
        )?;
    }
    writeln!(table)?;
    let crval = r["metadata"]["crval3"].as_f64().ok_or("crval")?;
    let crpix = r["metadata"]["crpix3"].as_f64().ok_or("crpix")?;
    let step = r["metadata"]["step_um"].as_f64().ok_or("step")?;
    for k in 0..nz {
        let wave = crval + ((k + 1) as f64 - crpix) * step;
        write!(table, "{k},{wave}")?;
        for (selected, sky) in &geometry_list {
            let values: Vec<_> = selected
                .iter()
                .filter_map(|xy| sample(&b, xy * nz + k))
                .collect();
            let mut sky_values: Vec<_> = sky
                .iter()
                .filter_map(|xy| sample(&b, xy * nz + k))
                .collect();
            let (n, ns) = (values.len(), sky_values.len());
            let enough = n as f64 >= 0.8 * selected.len() as f64
                && ns as f64 >= 20f64.min(0.5 * sky.len() as f64);
            let (flux, bg) = if enough && !values.is_empty() && !sky_values.is_empty() {
                let med = median(&mut sky_values);
                let flux = compensated_sum(values.iter().map(|v| (v - med) * conversion));
                require(flux.is_finite() && med.is_finite(), "Nonfinite aggregation")?;
                (Some(flux), Some(med))
            } else {
                (None, None)
            };
            write!(table, ",{},{},{n},{ns}", finite_text(flux), finite_text(bg))?;
        }
        writeln!(table)?;
    }
    table.flush()?;
    drop(table);
    let output = fs::read(out.join("rust_matched_apertures.csv"))?;
    let info = json!({"status":"GENERATED_PENDING_COMPARISON","protocol_sha256":a[3],"input_receipt_sha256":p["input_receipt_sha256"],"bin_sha256":hash(&b),"output_sha256":hash(&output),"conversion_njy_per_mjy_sr":conversion,"apertures":descriptions,"source_sha256":hash(include_bytes!("main.rs")),"code_scope":"Independent Rust regional reductions over frozen Goblin++-backend cube extraction; not a shipped language builtin or covariance inference"});
    create(&out.join("rust_regional_receipt.json"))?
        .write_all(&serde_json::to_vec_pretty(&info)?)?;
    println!("{}", serde_json::to_string_pretty(&info)?);
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shape_bounds_and_overflow() {
        assert_eq!(
            checked_shape(&json!([53, 51, 941])).unwrap(),
            (53, 51, 941, 50_870_460)
        );
        for dimensions in [
            json!([]),
            json!([1, 1]),
            json!([0, 2, 3]),
            json!([-1, 2, 3]),
            json!([1, 1, 10_000_001]),
            json!([u64::MAX, 2, 2]),
        ] {
            assert!(checked_shape(&dimensions).is_err());
        }
    }
    #[test]
    fn median_known() {
        assert_eq!(median(&mut [3., 1., 2.]), 2.);
        assert_eq!(median(&mut [4., 1., 3., 2.]), 2.5);
    }
    #[test]
    fn geometry_boundaries() {
        let (a, s) = geometry(5, 5, 2., 2., 1., [1., 2.]);
        assert_eq!(a, vec![12]);
        assert_eq!(s.len(), 8);
        assert!(!s.contains(&12));
    }
    #[test]
    fn preserve_negative_flux() {
        let flux = compensated_sum([1., 2.].iter().map(|x| (x - 3.) * 2.));
        assert_eq!(flux, -6.);
    }
    #[test]
    fn missing_and_valid_mask() {
        let mut b = [0u8; 20];
        b[0..4].copy_from_slice(&1f32.to_bits().to_le_bytes());
        b[12..16].copy_from_slice(&1f32.to_bits().to_le_bytes());
        assert_eq!(sample(&b, 0), Some(1.));
        b[8..12].copy_from_slice(&1u32.to_le_bytes());
        assert_eq!(sample(&b, 0), None);
    }
}
